//! Windows API 호출을 모아 둔 곳. 화면 코드는 여기 함수만 부른다.

use std::collections::BTreeMap;
use std::os::windows::process::CommandExt;
use std::process::Command;

use windows::Win32::Foundation::{CloseHandle, FILETIME, HWND, LPARAM, NO_ERROR, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetIfTable2, IF_TYPE_SOFTWARE_LOOPBACK, MIB_IF_ROW2, MIB_IF_TABLE2,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Power::{
    ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetSuspendState, SetThreadExecutionState,
};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::{
    GetSystemTimes, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE, QueryFullProcessImageNameW, TerminateProcess,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetWindow, GetWindowLongPtrW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CLOSE, WS_EX_TOOLWINDOW,
};
use windows::core::{BOOL, PWSTR};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 끄면 Windows 화면 자체가 망가지는 프로그램. 목록에서 빼고 끄지도 않는다.
const PROTECTED: &[&str] = &[
    "explorer.exe",
    "applicationframehost.exe",
    "textinputhost.exe",
    "shellexperiencehost.exe",
    "startmenuexperiencehost.exe",
    "searchhost.exe",
    "lockapp.exe",
    "dwm.exe",
    "csrss.exe",
    "winlogon.exe",
];

pub fn is_protected(exe: &str) -> bool {
    PROTECTED.contains(&exe.to_ascii_lowercase().as_str())
}

/// 목록에 보여 줄 프로그램 하나(실행 파일 이름으로 묶음).
#[derive(Clone)]
pub struct AppEntry {
    pub exe: String,
    pub title: String,
    pub windows: usize,
}

struct TopWindow {
    hwnd: HWND,
    pid: u32,
    title: String,
}

unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
    list.push(hwnd);
    BOOL(1)
}

/// 작업 표시줄에 보이는 종류의 창(보이고, 주인 창이 없고, 도구 창이 아니고, 숨겨진 UWP 창이 아님).
fn top_windows() -> Vec<TopWindow> {
    let mut all: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(enum_cb), LPARAM(&mut all as *mut Vec<HWND> as isize));
    }
    let me = std::process::id();
    let mut out = Vec::new();
    for hwnd in all {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                continue;
            }
            if let Ok(owner) = GetWindow(hwnd, GW_OWNER) {
                if !owner.is_invalid() {
                    continue;
                }
            }
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            if ex & WS_EX_TOOLWINDOW.0 != 0 {
                continue;
            }
            let mut cloaked: u32 = 0;
            if DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED,
                &mut cloaked as *mut u32 as *mut _,
                std::mem::size_of::<u32>() as u32,
            )
            .is_ok()
                && cloaked != 0
            {
                continue;
            }
            let mut buf = [0u16; 512];
            let n = GetWindowTextW(hwnd, &mut buf);
            let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 || pid == me {
                continue;
            }
            out.push(TopWindow { hwnd, pid, title });
        }
    }
    out
}

fn exe_name(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let r = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        r.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        Some(path.rsplit('\\').next().unwrap_or(&path).to_string())
    }
}

/// 창이 떠 있는 프로그램 목록. 실행 파일 이름 순.
pub fn list_apps() -> Vec<AppEntry> {
    let mut map: BTreeMap<String, AppEntry> = BTreeMap::new();
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for w in top_windows() {
        if w.title.trim().is_empty() {
            continue;
        }
        let Some(exe) = names.entry(w.pid).or_insert_with(|| exe_name(w.pid)).clone() else {
            continue;
        };
        if is_protected(&exe) {
            continue;
        }
        let e = map.entry(exe.to_ascii_lowercase()).or_insert_with(|| AppEntry {
            exe: exe.clone(),
            title: w.title.clone(),
            windows: 0,
        });
        e.windows += 1;
    }
    map.into_values().collect()
}

/// 실행 파일 이름이 같은 모든 프로세스 번호(창 없는 것 포함, 이 프로그램 제외).
pub fn pids_of(exe: &str) -> Vec<u32> {
    let me = std::process::id();
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return out;
        };
        let mut e = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snap, &mut e).is_ok() {
            loop {
                let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                let name = String::from_utf16_lossy(&e.szExeFile[..len]);
                if name.eq_ignore_ascii_case(exe) && e.th32ProcessID != me {
                    out.push(e.th32ProcessID);
                }
                if Process32NextW(snap, &mut e).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    out
}

/// 정상 종료 요청: 그 프로그램의 창마다 X를 누른 것과 같은 WM_CLOSE를 보낸다. 보낸 창 수를 돌려준다.
pub fn close_windows(exe: &str) -> usize {
    if is_protected(exe) {
        return 0;
    }
    let pids = pids_of(exe);
    let mut sent = 0;
    for w in top_windows() {
        if pids.contains(&w.pid) {
            unsafe {
                if PostMessageW(Some(w.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).is_ok() {
                    sent += 1;
                }
            }
        }
    }
    sent
}

/// 강제 종료. (끈 수, 못 끈 수)
pub fn kill(exe: &str) -> (usize, usize) {
    if is_protected(exe) {
        return (0, 0);
    }
    let (mut ok, mut fail) = (0, 0);
    for pid in pids_of(exe) {
        unsafe {
            match OpenProcess(PROCESS_TERMINATE, false, pid) {
                Ok(h) => {
                    if TerminateProcess(h, 1).is_ok() {
                        ok += 1;
                    } else {
                        fail += 1;
                    }
                    let _ = CloseHandle(h);
                }
                Err(_) => fail += 1,
            }
        }
    }
    (ok, fail)
}

/// 타이머가 도는 동안 PC가 저절로 절전에 들어가지 않게 한다(이 프로그램이 켜져 있는 동안만).
pub fn keep_awake(on: bool) {
    unsafe {
        let flags = if on { ES_CONTINUOUS | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS };
        SetThreadExecutionState(flags);
    }
}

/// 지금까지 받은 바이트 수의 합(켜져 있는 실제 네트워크 장치만).
/// 필터·가상 장치는 같은 데이터를 한 번 더 세므로 빼고, 실제 장치가 없으면 루프백만 뺀다.
pub fn net_recv_bytes() -> Option<u64> {
    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if GetIfTable2(&mut table) != NO_ERROR || table.is_null() {
            return None;
        }
        let n = (*table).NumEntries as usize;
        let rows: &[MIB_IF_ROW2] = std::slice::from_raw_parts((*table).Table.as_ptr(), n);
        let up = |r: &&MIB_IF_ROW2| r.OperStatus == IfOperStatusUp && r.Type != IF_TYPE_SOFTWARE_LOOPBACK;
        // 비트 0: 실제 장치(HardwareInterface), 비트 1: 필터(FilterInterface)
        let hw = |r: &&MIB_IF_ROW2| {
            let f = r.InterfaceAndOperStatusFlags._bitfield;
            f & 0b01 != 0 && f & 0b10 == 0
        };
        let mut sum: u64 = rows.iter().filter(up).filter(hw).map(|r| r.InOctets).sum();
        if !rows.iter().filter(up).any(|r| hw(&r)) {
            sum = rows
                .iter()
                .filter(up)
                .filter(|r| r.InterfaceAndOperStatusFlags._bitfield & 0b10 == 0)
                .map(|r| r.InOctets)
                .sum();
        }
        FreeMibTable(table as *const _);
        Some(sum)
    }
}

/// CPU 누적 시간 (쉰 시간, 전체 시간). 100ns 단위. 두 번 읽은 차이로 사용률을 낸다.
pub fn cpu_times() -> Option<(u64, u64)> {
    let ft = |f: FILETIME| (f.dwHighDateTime as u64) << 32 | f.dwLowDateTime as u64;
    let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).ok()? };
    // 커널 시간에는 쉰 시간이 들어 있다.
    Some((ft(idle), ft(kernel) + ft(user)))
}

/// 지금 지역 시각을 자정부터의 초로.
pub fn local_secs_of_day() -> u32 {
    let t = unsafe { GetLocalTime() };
    t.wHour as u32 * 3600 + t.wMinute as u32 * 60 + t.wSecond as u32
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PcAction {
    Shutdown,
    Restart,
    Sleep,
    Hibernate,
}

impl PcAction {
    pub const ALL: [PcAction; 4] = [Self::Shutdown, Self::Restart, Self::Sleep, Self::Hibernate];

    pub fn label(self) -> &'static str {
        match self {
            Self::Shutdown => "종료",
            Self::Restart => "다시 시작",
            Self::Sleep => "절전",
            Self::Hibernate => "최대 절전",
        }
    }

    /// 강제 옵션이 의미 있는 동작인지(절전은 프로그램을 닫지 않는다).
    pub fn can_force(self) -> bool {
        matches!(self, Self::Shutdown | Self::Restart)
    }
}

fn shutdown_exe() -> String {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    format!(r"{root}\System32\shutdown.exe")
}

/// PC 동작 실행.
pub fn run_power(action: PcAction, force: bool) -> Result<(), String> {
    // 시험용: 환경 변수 OFF_TIMER_DRY_RUN=1이면 실제로 끄지 않는다.
    if std::env::var("OFF_TIMER_DRY_RUN").is_ok_and(|v| v == "1") {
        return Err(format!(
            "시험 모드: 실제로는 {}{}하지 않았어요",
            action.label(),
            if force && action.can_force() { "(강제)" } else { "" }
        ));
    }
    if action == PcAction::Sleep {
        let ok = unsafe { SetSuspendState(false, false, false) };
        return if ok { Ok(()) } else { Err("절전에 들어가지 못했어요".into()) };
    }
    let mut args: Vec<&str> = match action {
        PcAction::Shutdown => vec!["/s", "/t", "0"],
        PcAction::Restart => vec!["/r", "/t", "0"],
        PcAction::Hibernate => vec!["/h"],
        PcAction::Sleep => unreachable!(),
    };
    if force && action.can_force() {
        args.push("/f");
    }
    let status = Command::new(shutdown_exe())
        .args(&args)
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("shutdown.exe를 실행하지 못했어요: {e}"))?;
    if status.success() {
        Ok(())
    } else if action == PcAction::Hibernate {
        Err("최대 절전이 꺼져 있는 것 같아요 (관리자 터미널에서 powercfg /h on)".into())
    } else {
        Err(format!("shutdown.exe가 실패했어요 (코드 {})", status.code().unwrap_or(-1)))
    }
}
