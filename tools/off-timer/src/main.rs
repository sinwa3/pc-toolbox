#![windows_subsystem = "windows"]
//! 끄기 타이머: 고른 프로그램과 PC를 정한 시간 뒤에(또는 조건이 되면) 끈다.

mod win;

use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime};

use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Frame,
    Layout, RichText, ScrollArea, Stroke, TextEdit, ViewportBuilder, ViewportCommand,
    vec2,
};
use win::{AppEntry, PcAction};

/// 정상 종료 요청 뒤 강제 종료까지 기다리는 시간.
const GRACE_SECS: u64 = 30;
/// PC 동작 전 경고 창을 띄우는 시간.
const WARN_SECS: u64 = 60;
/// 경고 창의 "미루기" 시간.
const SNOOZE_SECS: u64 = 10 * 60;
/// 빠른 버튼(분).
const QUICK: [(&str, u64); 3] = [("30분", 30), ("1시간", 60), ("2시간", 120)];
/// 조건 끄기에서 받는 속도·CPU 사용률을 평균 내는 구간(초). 잠깐 튀는 값에 흔들리지 않게 한다.
const AVG_SECS: f64 = 10.0;
/// 조건 끄기 "프로그램이 꺼지면"에서 사라진 뒤 다시 확인하는 시간(스스로 다시 켜지는 설치 프로그램 대비).
const GONE_SECS: u64 = 5;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("끄기 타이머")
            .with_inner_size([520.0, 760.0])
            .with_min_inner_size([440.0, 560.0])
            .with_icon(egui::IconData {
                rgba: include_bytes!("../assets/icon-64.rgba").to_vec(),
                width: 64,
                height: 64,
            }),
        ..Default::default()
    };
    eframe::run_native(
        "off-timer",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            setup_style(&cc.egui_ctx);
            Ok(Box::new(OffTimer::new()))
        }),
    )
}

// ---------- 글꼴·모양 ----------

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let dir = format!(r"{}\Fonts\", std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into()));
    let mut bold = Vec::new();
    if let Ok(bytes) = std::fs::read(format!("{dir}malgunbd.ttf")) {
        fonts.font_data.insert("malgun-bold".into(), FontData::from_owned(bytes).into());
        bold.push("malgun-bold".to_string());
    }
    if let Ok(bytes) = std::fs::read(format!("{dir}malgun.ttf")) {
        fonts.font_data.insert("malgun".into(), FontData::from_owned(bytes).into());
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "malgun".into());
        fonts.families.entry(FontFamily::Monospace).or_default().push("malgun".into());
    }
    bold.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
    fonts.families.insert(FontFamily::Name("bold".into()), bold);
    ctx.set_fonts(fonts);
}

fn setup_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.spacing.button_padding = vec2(10.0, 5.0);
        s.spacing.interact_size.y = 28.0;
        for (_, font) in s.text_styles.iter_mut() {
            font.size = (font.size + 1.5).round();
        }
        let r = CornerRadius::same(6);
        let w = &mut s.visuals.widgets;
        for st in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
            st.corner_radius = r;
        }
    });
}

fn accent(ui: &egui::Ui) -> (Color32, Color32) {
    // Windows 11 기본 강조색 (배경, 글자)
    if ui.visuals().dark_mode {
        (Color32::from_rgb(0x60, 0xCD, 0xFF), Color32::BLACK)
    } else {
        (Color32::from_rgb(0x00, 0x5F, 0xB8), Color32::WHITE)
    }
}

fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let (bg, fg) = accent(ui);
    ui.add(Button::new(RichText::new(text).color(fg).strong()).fill(bg).stroke(Stroke::NONE))
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let v = ui.visuals();
    Frame::new()
        .fill(v.faint_bg_color)
        .stroke(v.widgets.noninteractive.bg_stroke)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn bold(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text).family(FontFamily::Name("bold".into())).size(size).strong()
}

fn ok_color(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode { Color32::from_rgb(0x6C, 0xCB, 0x5F) } else { Color32::from_rgb(0x0F, 0x7B, 0x0F) }
}

fn warn_color(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode { Color32::from_rgb(0xFC, 0xE1, 0x00) } else { Color32::from_rgb(0x9D, 0x5D, 0x00) }
}

// ---------- 시간 ----------

/// "90"(분) 또는 "02:30"(시각, 지났으면 내일) → 지금부터 몇 초 뒤인지.
fn parse_when(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("시간을 넣어 주세요".into());
    }
    if let Some((h, m)) = s.split_once(':') {
        let h: u64 = h.trim().parse().map_err(|_| "시각은 02:30처럼 넣어 주세요")?;
        let m: u64 = m.trim().parse().map_err(|_| "시각은 02:30처럼 넣어 주세요")?;
        if h > 23 || m > 59 {
            return Err("시각은 00:00 ~ 23:59 사이로 넣어 주세요".into());
        }
        let now = win::local_secs_of_day() as u64;
        let target = h * 3600 + m * 60;
        return Ok(if target > now { target - now } else { target + 86_400 - now });
    }
    let min: u64 = s.parse().map_err(|_| "분은 숫자로, 시각은 02:30처럼 넣어 주세요")?;
    if min == 0 || min > 7 * 24 * 60 {
        return Err("1분 ~ 7일 사이로 넣어 주세요".into());
    }
    Ok(min * 60)
}

/// 지금부터 secs초 뒤의 시각을 "오후 11:42" 꼴로.
fn clock_after(secs: u64) -> String {
    let t = win::local_secs_of_day() as u64 + secs;
    let (day, sod) = (t / 86_400, t % 86_400);
    let (h, m) = (sod / 3600, sod % 3600 / 60);
    let ampm = if h < 12 { "오전" } else { "오후" };
    let h12 = if h % 12 == 0 { 12 } else { h % 12 };
    let prefix = match day {
        0 => String::new(),
        1 => "내일 ".into(),
        d => format!("{d}일 뒤 "),
    };
    format!("{prefix}{ampm} {h12}:{m:02}")
}

fn fmt_left(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

fn secs_until(deadline: SystemTime, now: SystemTime) -> u64 {
    deadline.duration_since(now).map(|d| d.as_secs_f64().ceil() as u64).unwrap_or(0)
}

fn secs_since(since: SystemTime, now: SystemTime) -> u64 {
    now.duration_since(since).map(|d| d.as_secs()).unwrap_or(0)
}

/// 바이트/초 → "820KB/s", "3.2MB/s"
fn fmt_rate(bps: f64) -> String {
    let kb = bps / 1024.0;
    if kb < 1000.0 { format!("{kb:.0}KB/s") } else { format!("{:.1}MB/s", kb / 1024.0) }
}

/// 프로그램 이름 입력 → 실행 파일 이름("ffmpeg" → "ffmpeg.exe").
fn exe_input(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() || s.contains('.') { s.to_string() } else { format!("{s}.exe") }
}

/// 숫자 입력칸 값 → 범위 안의 숫자.
fn parse_num(s: &str, min: f64, max: f64, what: &str) -> Result<f64, String> {
    match s.trim().parse::<f64>() {
        Ok(v) if v.is_finite() && v >= min && v <= max => Ok(v),
        _ => Err(format!("{what}은(는) {min} ~ {max} 사이 숫자로 넣어 주세요")),
    }
}

// ---------- 조건 끄기 ----------

/// 받는 속도·CPU 사용률을 1초마다 읽어 최근 AVG_SECS초 평균을 낸다.
struct Sampler {
    samples: VecDeque<(Instant, Option<u64>, Option<(u64, u64)>)>,
}

impl Sampler {
    fn new() -> Self {
        let mut s = Self { samples: VecDeque::new() };
        s.tick();
        s
    }

    /// 1초가 지났으면 새로 읽는다. 새로 읽었으면 true.
    fn tick(&mut self) -> bool {
        let now = Instant::now();
        if self.samples.back().is_some_and(|(t, ..)| now.duration_since(*t) < Duration::from_secs(1)) {
            return false;
        }
        self.samples.push_back((now, win::net_recv_bytes(), win::cpu_times()));
        // 평균 구간보다 오래된 표본은 기준점 하나만 남긴다.
        while self.samples.len() > 2 && now.duration_since(self.samples[1].0).as_secs_f64() >= AVG_SECS {
            self.samples.pop_front();
        }
        true
    }

    fn ends(&self) -> Option<(&(Instant, Option<u64>, Option<(u64, u64)>), &(Instant, Option<u64>, Option<(u64, u64)>))> {
        let (a, b) = (self.samples.front()?, self.samples.back()?);
        (b.0.duration_since(a.0).as_secs_f64() >= 0.9).then_some((a, b))
    }

    /// 최근 받는 속도(바이트/초).
    fn net_bps(&self) -> Option<f64> {
        let (a, b) = self.ends()?;
        let dt = b.0.duration_since(a.0).as_secs_f64();
        Some(b.1?.saturating_sub(a.1?) as f64 / dt)
    }

    /// 최근 CPU 사용률(%).
    fn cpu_pct(&self) -> Option<f64> {
        let (a, b) = self.ends()?;
        let ((i0, t0), (i1, t1)) = (a.2?, b.2?);
        let total = t1.saturating_sub(t0);
        if total == 0 {
            return None;
        }
        let idle = i1.saturating_sub(i0).min(total);
        Some(100.0 * (total - idle) as f64 / total as f64)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PcMode {
    Time,
    Cond,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cond {
    Net,
    Cpu,
    Exit,
}

impl Cond {
    const ALL: [Cond; 3] = [Self::Net, Self::Cpu, Self::Exit];

    fn label(self) -> &'static str {
        match self {
            Self::Net => "다운로드가 끝나면",
            Self::Cpu => "CPU가 한가해지면",
            Self::Exit => "프로그램이 꺼지면",
        }
    }
}

/// 돌고 있는 조건 하나.
struct CondRun {
    cond: Cond,
    /// 바이트/초(Net) 또는 %(Cpu). 이보다 낮으면 조용하다고 본다.
    limit: f64,
    quiet_secs: u64,
    exe: String,
    /// 기준을 한 번 넘었는지. "넘은 뒤부터 세기"를 끄면 처음부터 true.
    busy_seen: bool,
    quiet_since: Option<SystemTime>,
    /// "10분 미루기"를 눌렀으면 이때까지는 경고하지 않는다.
    hold_until: Option<SystemTime>,
}

impl CondRun {
    /// 지금 값(Net·Cpu). 아직 평균을 낼 표본이 모자라면 None.
    fn value(&self, s: &Sampler) -> Option<f64> {
        match self.cond {
            Cond::Net => s.net_bps(),
            Cond::Cpu => s.cpu_pct(),
            Cond::Exit => None,
        }
    }

    fn need_secs(&self) -> u64 {
        if self.cond == Cond::Exit { GONE_SECS } else { self.quiet_secs }
    }

    /// 1초마다 부른다. 경고로 넘어갈 때가 되면 true.
    fn check(&mut self, now: SystemTime, s: &Sampler) -> bool {
        let quiet = match self.cond {
            Cond::Exit => win::pids_of(&self.exe).is_empty(),
            _ => match self.value(s) {
                Some(v) => v < self.limit,
                None => return false,
            },
        };
        if !quiet {
            self.busy_seen = true;
            self.quiet_since = None;
            return false;
        }
        if !self.busy_seen {
            return false;
        }
        let since = *self.quiet_since.get_or_insert(now);
        let held = self.hold_until.is_some_and(|h| now < h);
        !held && secs_since(since, now) >= self.need_secs()
    }

    /// 경고 창에 보여 줄 이유.
    fn reason(&self) -> String {
        let min = self.quiet_secs / 60;
        match self.cond {
            Cond::Net => format!("받는 속도가 {min}분 동안 낮았어요"),
            Cond::Cpu => format!("CPU 사용률이 {min}분 동안 낮았어요"),
            Cond::Exit => format!("{}가 꺼졌어요", self.exe),
        }
    }
}

// ---------- 상태 ----------

enum ProgState {
    Waiting,
    Closing { since: SystemTime },
    Done(String),
    Failed(String),
}

struct ProgTimer {
    id: u64,
    exe: String,
    deadline: SystemTime,
    force: bool,
    state: ProgState,
}

impl ProgTimer {
    fn active(&self) -> bool {
        matches!(self.state, ProgState::Waiting | ProgState::Closing { .. })
    }
}

struct PcRun {
    action: PcAction,
    force: bool,
    /// 시간 끄기의 마감, 또는 (조건 끄기에서) 경고가 끝나는 시각.
    deadline: SystemTime,
    warning: bool,
    /// 조건 끄기면 Some. 경고 전까지는 deadline을 쓰지 않는다.
    cond: Option<CondRun>,
}

struct OffTimer {
    apps: Vec<AppEntry>,
    last_refresh: SystemTime,
    search: String,
    selected: Option<String>,
    prog_when: String,
    prog_force: bool,
    prog_msg: Option<String>,
    timers: Vec<ProgTimer>,
    next_id: u64,

    pc_action: PcAction,
    pc_mode: PcMode,
    pc_when: String,
    pc_force: bool,
    pc: Option<PcRun>,
    pc_msg: Option<String>,

    cond: Cond,
    cond_net_kb: String,
    cond_cpu_pct: String,
    cond_quiet_min: String,
    cond_wait_busy: bool,
    cond_exe: String,
    /// 조건 입력칸의 프로그램이 지금 몇 개 떠 있는지(1초마다 갱신).
    cond_exe_count: usize,
    sampler: Sampler,

    confirm_exit: bool,
    allow_exit: bool,
    awake: bool,
    raised: bool,
}

impl OffTimer {
    fn new() -> Self {
        Self {
            apps: win::list_apps(),
            last_refresh: SystemTime::now(),
            search: String::new(),
            selected: None,
            prog_when: "60".into(),
            prog_force: true,
            prog_msg: None,
            timers: Vec::new(),
            next_id: 1,
            pc_action: PcAction::Shutdown,
            pc_mode: PcMode::Time,
            pc_when: "120".into(),
            pc_force: false,
            pc: None,
            pc_msg: None,
            cond: Cond::Net,
            cond_net_kb: "100".into(),
            cond_cpu_pct: "10".into(),
            cond_quiet_min: "5".into(),
            cond_wait_busy: true,
            cond_exe: String::new(),
            cond_exe_count: 0,
            sampler: Sampler::new(),
            confirm_exit: false,
            allow_exit: false,
            awake: false,
            raised: false,
        }
    }

    fn any_active(&self) -> bool {
        self.pc.is_some() || self.timers.iter().any(ProgTimer::active)
    }

    fn refresh_apps(&mut self) {
        self.apps = win::list_apps();
        self.last_refresh = SystemTime::now();
    }

    fn tick_programs(&mut self, now: SystemTime) {
        for t in &mut self.timers {
            match t.state {
                ProgState::Waiting if now >= t.deadline => {
                    if win::pids_of(&t.exe).is_empty() {
                        t.state = ProgState::Done("이미 꺼져 있었어요".into());
                    } else {
                        win::close_windows(&t.exe);
                        t.state = ProgState::Closing { since: now };
                    }
                }
                ProgState::Closing { since } => {
                    if win::pids_of(&t.exe).is_empty() {
                        t.state = ProgState::Done("꺼졌어요".into());
                    } else if now.duration_since(since).unwrap_or_default().as_secs() >= GRACE_SECS {
                        t.state = if t.force {
                            force_result(&t.exe)
                        } else {
                            ProgState::Failed("안 꺼졌어요. 저장 창이 떠 있는지 확인해 주세요".into())
                        };
                    }
                }
                _ => {}
            }
        }
    }

    fn tick_pc(&mut self, now: SystemTime, fresh: bool) {
        let Some(run) = &mut self.pc else { return };
        if !run.warning {
            if let Some(c) = &mut run.cond {
                if fresh && c.check(now, &self.sampler) {
                    run.warning = true;
                    run.deadline = now + Duration::from_secs(WARN_SECS);
                }
                return;
            }
            if secs_until(run.deadline, now) <= WARN_SECS {
                run.warning = true;
                // 절전 등으로 시간을 건너뛰어 이미 지났으면 경고 시간을 새로 준다.
                if now > run.deadline + Duration::from_secs(5) {
                    run.deadline = now + Duration::from_secs(WARN_SECS);
                }
            }
            return;
        }
        if now >= run.deadline {
            let (action, force) = (run.action, run.force);
            self.pc = None;
            self.pc_msg = win::run_power(action, force).err();
        }
    }
}

fn force_result(exe: &str) -> ProgState {
    match win::kill(exe) {
        (_, 0) => ProgState::Done("강제로 껐어요".into()),
        (0, _) => ProgState::Failed("끄지 못했어요 (관리자 권한으로 실행된 프로그램일 수 있어요)".into()),
        (_, _) => ProgState::Failed("일부를 끄지 못했어요 (관리자 권한 프로그램일 수 있어요)".into()),
    }
}

impl eframe::App for OffTimer {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = SystemTime::now();
        let fresh = self.sampler.tick();
        if fresh && self.pc_mode == PcMode::Cond && self.cond == Cond::Exit {
            self.cond_exe_count = win::pids_of(&exe_input(&self.cond_exe)).len();
        }
        self.tick_programs(now);
        self.tick_pc(now, fresh);

        let active = self.any_active();
        if active != self.awake {
            win::keep_awake(active);
            self.awake = active;
        }

        if now.duration_since(self.last_refresh).unwrap_or_default() >= Duration::from_secs(3) {
            self.refresh_apps();
        }

        // 타이머가 있는데 창을 닫으려 하면 먼저 묻는다.
        if ctx.input(|i| i.viewport().close_requested()) && active && !self.allow_exit {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
            self.confirm_exit = true;
        }

        // 경고 시간이 되면 창을 다시 띄워 맨 위에 둔다. 경고가 끝나면 원래대로.
        // (내려 둔 창에서 별도 경고 창을 만들면 egui 0.37이 멈춰서 같은 창 안에 경고를 띄운다.)
        let warning = self.pc.as_ref().is_some_and(|r| r.warning);
        if warning && !self.raised {
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
            ctx.send_viewport_cmd(ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
            self.raised = true;
        } else if !warning && self.raised {
            ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
            ctx.send_viewport_cmd(ViewportCommand::RequestUserAttention(egui::UserAttentionType::Reset));
            self.raised = false;
        }

        ctx.request_repaint_after(Duration::from_millis(if active { 250 } else { 1000 }));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.add_space(4.0);
                card(ui, |ui| self.programs_ui(ui));
                ui.add_space(6.0);
                card(ui, |ui| self.pc_ui(ui));
            });
        });
        self.warning_ui(ui.ctx());
        self.exit_ui(ui.ctx());
    }
}

// ---------- 화면 ----------

/// 안내·오류 한 줄과 닫기 버튼.
fn msg_line(ui: &mut egui::Ui, msg: &mut Option<String>) {
    let Some(text) = msg.clone() else { return };
    let c = warn_color(ui);
    ui.horizontal(|ui| {
        ui.label(RichText::new(text).color(c));
        if ui.small_button("닫기").clicked() {
            *msg = None;
        }
    });
}

/// 30분/1시간/2시간 버튼 + 직접 입력칸 + 언제인지 미리보기.
fn when_ui(ui: &mut egui::Ui, value: &mut String, verb: &str) -> Option<u64> {
    ui.horizontal(|ui| {
        for (label, min) in QUICK {
            let on = value.trim() == min.to_string();
            if ui.add(Button::new(label).selected(on)).clicked() {
                *value = min.to_string();
            }
        }
        ui.add(
            TextEdit::singleline(value)
                .hint_text("분 또는 02:30")
                .desired_width(ui.available_width()),
        );
    });
    match parse_when(value) {
        Ok(secs) => {
            ui.label(RichText::new(format!("→ {}에 {verb}", clock_after(secs))).weak());
            Some(secs)
        }
        Err(e) => {
            ui.label(RichText::new(e).color(warn_color(ui)));
            None
        }
    }
}

impl OffTimer {
    fn programs_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(bold("프로그램 끄기", 17.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("새로고침").clicked() {
                    self.refresh_apps();
                }
            });
        });
        ui.add(
            TextEdit::singleline(&mut self.search)
                .hint_text("프로그램 검색")
                .desired_width(f32::INFINITY),
        );

        let q = self.search.trim().to_lowercase();
        let list: Vec<&AppEntry> = self
            .apps
            .iter()
            .filter(|a| q.is_empty() || a.exe.to_lowercase().contains(&q) || a.title.to_lowercase().contains(&q))
            .collect();

        Frame::new()
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(4)
            .show(ui, |ui| {
                ScrollArea::vertical().id_salt("apps").max_height(170.0).auto_shrink(false).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if list.is_empty() {
                        ui.label(RichText::new("창이 떠 있는 프로그램이 없어요").weak());
                    }
                    for a in list {
                        let sel = self.selected.as_deref().is_some_and(|s| s.eq_ignore_ascii_case(&a.exe));
                        let mut text = egui::text::LayoutJob::default();
                        let style = ui.style().clone();
                        RichText::new(&a.exe).strong().append_to(&mut text, &style, egui::FontSelection::Default, Align::Center);
                        let extra = if a.windows > 1 { format!("  창 {}개 · {}", a.windows, a.title) } else { format!("  {}", a.title) };
                        RichText::new(extra).weak().append_to(&mut text, &style, egui::FontSelection::Default, Align::Center);
                        let r = ui.add(
                            Button::selectable(sel, text)
                                .truncate()
                                .min_size(vec2(ui.available_width(), 26.0)),
                        );
                        if r.clicked() {
                            self.selected = Some(a.exe.clone());
                        }
                    }
                });
            });

        let secs = when_ui(ui, &mut self.prog_when, "꺼져요");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.prog_force, format!("{GRACE_SECS}초 안에 안 꺼지면 강제 종료"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let label = match &self.selected {
                    Some(exe) => format!("{exe} 추가"),
                    None => "추가".into(),
                };
                if primary(ui, &label).clicked() {
                    match (&self.selected, secs) {
                        (Some(exe), Some(secs)) => {
                            self.timers.push(ProgTimer {
                                id: self.next_id,
                                exe: exe.clone(),
                                deadline: SystemTime::now() + Duration::from_secs(secs),
                                force: self.prog_force,
                                state: ProgState::Waiting,
                            });
                            self.next_id += 1;
                        }
                        (None, _) => self.prog_msg = Some("위 목록에서 끌 프로그램을 골라 주세요".into()),
                        _ => {}
                    }
                }
            });
        });
        if self.selected.is_some() && self.prog_msg.is_some() {
            self.prog_msg = None;
        }
        msg_line(ui, &mut self.prog_msg);

        if !self.timers.is_empty() {
            ui.separator();
        }
        let now = SystemTime::now();
        let mut remove = None;
        let mut force_now = None;
        for t in &self.timers {
            Frame::new()
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(10, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&t.exe).strong());
                        if !t.force {
                            ui.label(RichText::new("강제 안 함").weak().small());
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| match &t.state {
                            ProgState::Waiting => {
                                if ui.button("취소").clicked() {
                                    remove = Some(t.id);
                                }
                                ui.label(bold(fmt_left(secs_until(t.deadline, now)), 16.0));
                            }
                            ProgState::Closing { since } => {
                                if ui.button("취소").clicked() {
                                    remove = Some(t.id);
                                }
                                let waited = now.duration_since(*since).unwrap_or_default().as_secs();
                                let txt = if t.force {
                                    format!("끄는 중… {}초 뒤 강제", GRACE_SECS.saturating_sub(waited))
                                } else {
                                    "끄는 중…".into()
                                };
                                ui.label(RichText::new(txt).weak());
                            }
                            ProgState::Done(msg) => {
                                if ui.button("지우기").clicked() {
                                    remove = Some(t.id);
                                }
                                ui.label(RichText::new(msg).color(ok_color(ui)));
                            }
                            ProgState::Failed(msg) => {
                                if ui.button("지우기").clicked() {
                                    remove = Some(t.id);
                                }
                                if ui.button("강제로 끄기").clicked() {
                                    force_now = Some(t.id);
                                }
                                ui.add(egui::Label::new(RichText::new(msg).color(warn_color(ui))).truncate());
                            }
                        });
                    });
                });
        }
        if let Some(id) = remove {
            self.timers.retain(|t| t.id != id);
        }
        if let Some(id) = force_now {
            if let Some(t) = self.timers.iter_mut().find(|t| t.id == id) {
                t.state = force_result(&t.exe);
            }
        }
    }

    fn pc_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(bold("PC 끄기", 17.0));
        let now = SystemTime::now();

        if self.pc.is_some() {
            self.pc_running_ui(ui, now);
        } else {
            ui.horizontal(|ui| {
                for (m, label) in [(PcMode::Time, "시간 뒤에"), (PcMode::Cond, "조건이 되면")] {
                    if ui.add(Button::new(label).selected(self.pc_mode == m)).clicked() {
                        self.pc_mode = m;
                    }
                }
            });
            ui.horizontal(|ui| {
                for a in PcAction::ALL {
                    if ui.add(Button::new(a.label()).selected(self.pc_action == a)).clicked() {
                        self.pc_action = a;
                    }
                }
            });
            let verb = match self.pc_action {
                PcAction::Shutdown => "꺼져요",
                PcAction::Restart => "다시 시작해요",
                PcAction::Sleep => "절전에 들어가요",
                PcAction::Hibernate => "최대 절전에 들어가요",
            };
            // 시작했을 때 만들 것: (마감까지 초, 조건)
            let ready: Option<(u64, Option<CondRun>)> = match self.pc_mode {
                PcMode::Time => when_ui(ui, &mut self.pc_when, verb).map(|s| (s, None)),
                PcMode::Cond => self.cond_ui(ui).map(|c| (0, Some(c))),
            };
            ui.horizontal(|ui| {
                if self.pc_action.can_force() {
                    ui.checkbox(&mut self.pc_force, "저장 안 된 프로그램이 있어도 끄기");
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary(ui, "시작").clicked() {
                        if let Some((secs, cond)) = ready {
                            self.pc = Some(PcRun {
                                action: self.pc_action,
                                force: self.pc_force && self.pc_action.can_force(),
                                deadline: SystemTime::now() + Duration::from_secs(secs),
                                warning: false,
                                cond,
                            });
                            self.pc_msg = None;
                        }
                    }
                });
            });
        }
        let cond_mode = match &self.pc {
            Some(run) => run.cond.is_some(),
            None => self.pc_mode == PcMode::Cond,
        };
        let note = if cond_mode {
            format!("조건이 맞으면 {WARN_SECS}초 동안 경고 창을 띄운 뒤에 실행해요. 기다리는 동안은 PC가 저절로 절전에 들어가지 않아요.")
        } else {
            format!("{WARN_SECS}초 전에 경고 창이 떠요. 타이머가 도는 동안은 PC가 저절로 절전에 들어가지 않아요.")
        };
        ui.label(RichText::new(note).weak().small());

        msg_line(ui, &mut self.pc_msg);
    }

    /// PC 타이머가 돌고 있을 때: 남은 시간 또는 조건 상태 + 취소.
    fn pc_running_ui(&mut self, ui: &mut egui::Ui, now: SystemTime) {
        let Some(run) = &self.pc else { return };
        let action = run.action.label();
        let (big, small): (String, String) = match &run.cond {
            None => {
                let left = secs_until(run.deadline, now);
                (fmt_left(left), format!("{}에 {action}", clock_after(left)))
            }
            Some(_) if run.warning => {
                let left = secs_until(run.deadline, now);
                (fmt_left(left), format!("조건이 맞아서 곧 {action}해요"))
            }
            Some(c) => {
                let need = c.need_secs();
                match (c.cond, c.busy_seen, c.quiet_since) {
                    (Cond::Exit, _, None) => ("기다리는 중".into(), format!("{}가 꺼지면 {action}", c.exe)),
                    (Cond::Exit, _, Some(_)) => ("확인 중".into(), format!("{}가 꺼졌어요. 다시 켜지지 않는지 보고 있어요", c.exe)),
                    (_, false, _) => (
                        "시작 기다리는 중".into(),
                        "기준을 한 번 넘은 뒤부터 조용한 시간을 세요".into(),
                    ),
                    (_, true, None) => (
                        if c.cond == Cond::Net { "받는 중".into() } else { "사용 중".into() },
                        format!("기준보다 낮은 상태가 {}분 이어지면 {action}", need / 60),
                    ),
                    (_, true, Some(since)) => (
                        fmt_left(need.saturating_sub(secs_since(since, now))),
                        format!("이대로 조용하면 {action}"),
                    ),
                }
            }
        };
        ui.horizontal(|ui| {
            ui.label(bold(big, 34.0));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("취소").clicked() {
                    self.pc = None;
                }
            });
        });
        ui.label(RichText::new(small).weak());
        let Some(run) = &self.pc else { return };
        if let Some(c) = &run.cond {
            let current = match c.cond {
                Cond::Net => c.value(&self.sampler).map(|v| format!("지금 받는 속도 {} · 기준 {} 미만", fmt_rate(v), fmt_rate(c.limit))),
                Cond::Cpu => c.value(&self.sampler).map(|v| format!("지금 CPU {v:.0}% · 기준 {:.0}% 미만", c.limit)),
                Cond::Exit => None,
            };
            if let Some(t) = current {
                ui.label(RichText::new(t).weak());
            }
            if let Some(h) = c.hold_until.filter(|h| now < *h) {
                ui.label(RichText::new(format!("미루는 중: {}까지는 실행하지 않아요", clock_after(secs_until(h, now)))).weak());
            }
        }
    }

    /// 조건 고르기와 기준값 입력. 올바르면 시작할 조건을 돌려준다.
    fn cond_ui(&mut self, ui: &mut egui::Ui) -> Option<CondRun> {
        ui.horizontal(|ui| {
            for c in Cond::ALL {
                if ui.add(Button::new(c.label()).selected(self.cond == c)).clicked() {
                    self.cond = c;
                    if c == Cond::Exit {
                        self.cond_exe_count = win::pids_of(&exe_input(&self.cond_exe)).len();
                    }
                }
            }
        });
        let num = |ui: &mut egui::Ui, v: &mut String| {
            ui.add(TextEdit::singleline(v).desired_width(52.0).align(egui::Align2::CENTER_TOP));
        };

        let result: Result<CondRun, String> = match self.cond {
            Cond::Net | Cond::Cpu => {
                let net = self.cond == Cond::Net;
                ui.horizontal(|ui| {
                    ui.label(if net { "받는 속도가" } else { "CPU 사용률이" });
                    num(ui, if net { &mut self.cond_net_kb } else { &mut self.cond_cpu_pct });
                    ui.label(if net { "KB/s 미만으로" } else { "% 미만으로" });
                    num(ui, &mut self.cond_quiet_min);
                    ui.label("분 이어지면");
                });
                let now_txt = if net {
                    self.sampler.net_bps().map(|v| format!("지금 받는 속도 {} (최근 10초 평균)", fmt_rate(v)))
                } else {
                    self.sampler.cpu_pct().map(|v| format!("지금 CPU {v:.0}% (최근 10초 평균)"))
                };
                ui.label(RichText::new(now_txt.unwrap_or_else(|| "지금 값을 읽는 중…".into())).weak());
                ui.checkbox(
                    &mut self.cond_wait_busy,
                    if net { "받는 속도가 기준을 넘은 뒤부터 세기" } else { "사용률이 기준을 넘은 뒤부터 세기" },
                )
                .on_hover_text("다운로드·작업을 걸기 전에 시작해도 바로 꺼지지 않아요");
                let limit = if net {
                    parse_num(&self.cond_net_kb, 1.0, 1_000_000.0, "받는 속도").map(|v| v * 1024.0)
                } else {
                    parse_num(&self.cond_cpu_pct, 1.0, 99.0, "CPU 사용률")
                };
                let quiet = parse_num(&self.cond_quiet_min, 1.0, 180.0, "이어지는 시간");
                limit.and_then(|limit| {
                    quiet.map(|q| CondRun {
                        cond: self.cond,
                        limit,
                        quiet_secs: q.round() as u64 * 60,
                        exe: String::new(),
                        busy_seen: !self.cond_wait_busy,
                        quiet_since: None,
                        hold_until: None,
                    })
                })
            }
            Cond::Exit => {
                ui.horizontal(|ui| {
                    ui.label("꺼지길 기다릴 프로그램");
                    let names: Vec<String> = self.apps.iter().map(|a| a.exe.clone()).collect();
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.menu_button("목록에서", |ui| {
                            ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                if names.is_empty() {
                                    ui.label(RichText::new("창이 떠 있는 프로그램이 없어요").weak());
                                }
                                for n in names {
                                    if ui.button(&n).clicked() {
                                        self.cond_exe_count = win::pids_of(&n).len();
                                        self.cond_exe = n;
                                        ui.close();
                                    }
                                }
                            });
                        });
                        ui.add(
                            TextEdit::singleline(&mut self.cond_exe)
                                .hint_text("예: setup.exe")
                                .desired_width(ui.available_width()),
                        );
                    });
                });
                ui.label(
                    RichText::new("창 없이 도는 프로그램(ffmpeg 등)은 이름을 직접 넣어 주세요. 이름이 같은 프로세스가 모두 꺼져야 해요.")
                        .weak()
                        .small(),
                );
                let exe = exe_input(&self.cond_exe);
                if exe.is_empty() {
                    Err("프로그램 이름을 넣어 주세요".into())
                } else if self.cond_exe_count == 0 {
                    Err(format!("{exe}는 지금 실행 중이 아니에요"))
                } else {
                    ui.label(RichText::new(format!("{exe} 실행 중 (프로세스 {}개)", self.cond_exe_count)).weak());
                    Ok(CondRun {
                        cond: Cond::Exit,
                        limit: 0.0,
                        quiet_secs: GONE_SECS,
                        exe,
                        busy_seen: true,
                        quiet_since: None,
                        hold_until: None,
                    })
                }
            }
        };
        match result {
            Ok(c) => Some(c),
            Err(e) => {
                ui.label(RichText::new(e).color(warn_color(ui)));
                None
            }
        }
    }

    fn warning_ui(&mut self, ctx: &egui::Context) {
        let Some(run) = &self.pc else { return };
        if !run.warning {
            return;
        }
        let left = secs_until(run.deadline, SystemTime::now());
        let what = match run.action {
            PcAction::Shutdown => "PC를 종료해요",
            PcAction::Restart => "PC를 다시 시작해요",
            PcAction::Sleep => "PC가 절전에 들어가요",
            PcAction::Hibernate => "PC가 최대 절전에 들어가요",
        };
        let reason = run.cond.as_ref().map(CondRun::reason);
        let (mut cancel, mut snooze) = (false, false);
        egui::Modal::new(egui::Id::unique("pc-warning")).show(ctx, |ui| {
            ui.set_width(300.0);
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
                if let Some(r) = &reason {
                    ui.label(RichText::new(r).weak());
                }
                ui.label(RichText::new(what).size(16.0));
                ui.label(bold(fmt_left(left), 44.0));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let w = 130.0;
                    ui.add_space((ui.available_width() - w * 2.0 - 8.0).max(0.0) / 2.0);
                    if ui.add_sized([w, 34.0], Button::new("10분 미루기")).clicked() {
                        snooze = true;
                    }
                    let (bg, fg) = accent(ui);
                    if ui
                        .add_sized([w, 34.0], Button::new(RichText::new("취소").color(fg).strong()).fill(bg).stroke(Stroke::NONE))
                        .clicked()
                    {
                        cancel = true;
                    }
                });
            });
        });
        if cancel {
            self.pc = None;
        } else if snooze {
            if let Some(run) = &mut self.pc {
                let now = SystemTime::now();
                run.deadline = now + Duration::from_secs(SNOOZE_SECS);
                run.warning = false;
                // 조건 끄기: 10분 동안은 실행하지 않고, 조용한 시간도 처음부터 다시 센다.
                if let Some(c) = &mut run.cond {
                    c.hold_until = Some(run.deadline);
                    c.quiet_since = None;
                }
            }
        }
    }

    fn exit_ui(&mut self, ctx: &egui::Context) {
        if !self.confirm_exit {
            return;
        }
        let modal = egui::Modal::new(egui::Id::unique("confirm-exit")).show(ctx, |ui| {
            ui.set_width(300.0);
            ui.label(bold("타이머가 남아 있어요", 16.0));
            ui.label("창을 닫으면 모든 타이머가 취소돼요.");
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("계속 두기").clicked() {
                    self.confirm_exit = false;
                }
                if ui.button("모두 취소하고 닫기").clicked() {
                    self.confirm_exit = false;
                    self.allow_exit = true;
                    self.pc = None;
                    self.timers.clear();
                    ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                }
            });
        });
        if modal.should_close() {
            self.confirm_exit = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 받는 바이트 수 기록으로 가짜 표본을 만든다(1초 간격).
    fn sampler_with(bytes: &[u64]) -> Sampler {
        let start = Instant::now();
        let samples = bytes
            .iter()
            .enumerate()
            .map(|(i, b)| (start + Duration::from_secs(i as u64), Some(*b), Some((0, 0))))
            .collect();
        Sampler { samples }
    }

    fn net_run(limit_kb: f64, quiet_min: u64, wait_busy: bool) -> CondRun {
        CondRun {
            cond: Cond::Net,
            limit: limit_kb * 1024.0,
            quiet_secs: quiet_min * 60,
            exe: String::new(),
            busy_seen: !wait_busy,
            quiet_since: None,
            hold_until: None,
        }
    }

    #[test]
    fn rate_is_average_over_window() {
        // 10초 동안 1MB씩 → 1MB/s
        let s = sampler_with(&(0..=10).map(|i| i * 1024 * 1024).collect::<Vec<_>>());
        assert!((s.net_bps().unwrap() - 1024.0 * 1024.0).abs() < 1.0);
        assert_eq!(fmt_rate(1024.0 * 1024.0), "1.0MB/s");
        assert_eq!(fmt_rate(100.0 * 1024.0), "100KB/s");
    }

    #[test]
    fn waits_for_busy_before_counting() {
        let quiet = sampler_with(&[0, 0, 0]);
        let busy = sampler_with(&[0, 10 * 1024 * 1024]);
        let t0 = SystemTime::now();
        let mut c = net_run(100.0, 5, true);
        // 처음부터 조용하면 세지 않는다.
        assert!(!c.check(t0, &quiet));
        assert!(!c.check(t0 + Duration::from_secs(600), &quiet));
        assert!(c.quiet_since.is_none());
        // 바빠진 뒤 조용해지면 그때부터 5분.
        assert!(!c.check(t0 + Duration::from_secs(601), &busy));
        assert!(c.busy_seen);
        let q0 = t0 + Duration::from_secs(602);
        assert!(!c.check(q0, &quiet));
        assert!(!c.check(q0 + Duration::from_secs(299), &quiet));
        assert!(c.check(q0 + Duration::from_secs(300), &quiet));
    }

    #[test]
    fn busy_resets_quiet_time() {
        let quiet = sampler_with(&[0, 0]);
        let busy = sampler_with(&[0, 10 * 1024 * 1024]);
        let t0 = SystemTime::now();
        let mut c = net_run(100.0, 5, false);
        assert!(!c.check(t0, &quiet));
        assert!(!c.check(t0 + Duration::from_secs(200), &busy));
        assert!(!c.check(t0 + Duration::from_secs(301), &quiet));
        assert!(!c.check(t0 + Duration::from_secs(600), &quiet));
        assert!(c.check(t0 + Duration::from_secs(601), &quiet));
    }

    #[test]
    fn hold_delays_trigger() {
        let quiet = sampler_with(&[0, 0]);
        let t0 = SystemTime::now();
        let mut c = net_run(100.0, 1, false);
        c.hold_until = Some(t0 + Duration::from_secs(600));
        assert!(!c.check(t0, &quiet));
        assert!(!c.check(t0 + Duration::from_secs(599), &quiet));
        assert!(c.check(t0 + Duration::from_secs(600), &quiet));
    }

    #[test]
    fn not_enough_samples_never_triggers() {
        let s = sampler_with(&[0]);
        let mut c = net_run(100.0, 1, false);
        assert!(s.net_bps().is_none());
        assert!(!c.check(SystemTime::now() + Duration::from_secs(3600), &s));
    }

    #[test]
    fn exe_input_adds_extension() {
        assert_eq!(exe_input(" ffmpeg "), "ffmpeg.exe");
        assert_eq!(exe_input("setup.exe"), "setup.exe");
        assert_eq!(exe_input(""), "");
    }

    #[test]
    fn real_counters_are_readable() {
        assert!(win::net_recv_bytes().is_some());
        let (idle, total) = win::cpu_times().unwrap();
        assert!(total >= idle && total > 0);
    }
}

#[cfg(test)]
mod live {
    use super::*;

    /// 실제 측정 확인용(수동): cargo test --release live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn print_live_rates() {
        let mut s = Sampler::new();
        let end = Instant::now() + Duration::from_secs(14);
        while Instant::now() < end {
            std::thread::sleep(Duration::from_millis(200));
            if s.tick() {
                println!(
                    "net {} cpu {:.0}%",
                    s.net_bps().map(fmt_rate).unwrap_or("-".into()),
                    s.cpu_pct().unwrap_or(-1.0)
                );
            }
        }
    }
}
