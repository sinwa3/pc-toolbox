#![windows_subsystem = "windows"]
//! 끄기 타이머: 고른 프로그램과 PC를 정한 시간 뒤에 끈다.

mod win;

use std::time::{Duration, SystemTime};

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

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("끄기 타이머")
            .with_inner_size([520.0, 660.0])
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
    deadline: SystemTime,
    warning: bool,
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
    pc_when: String,
    pc_force: bool,
    pc: Option<PcRun>,
    pc_msg: Option<String>,

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
            pc_when: "120".into(),
            pc_force: false,
            pc: None,
            pc_msg: None,
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

    fn tick_pc(&mut self, now: SystemTime) {
        let Some(run) = &mut self.pc else { return };
        if !run.warning {
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
        self.tick_programs(now);
        self.tick_pc(now);

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

        if let Some(run) = &self.pc {
            let left = secs_until(run.deadline, now);
            ui.horizontal(|ui| {
                ui.label(bold(fmt_left(left), 34.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("취소").clicked() {
                        self.pc = None;
                    }
                });
            });
            if let Some(run) = &self.pc {
                ui.label(RichText::new(format!("{}에 {}", clock_after(left), run.action.label())).weak());
            }
        } else {
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
            let secs = when_ui(ui, &mut self.pc_when, verb);
            ui.horizontal(|ui| {
                if self.pc_action.can_force() {
                    ui.checkbox(&mut self.pc_force, "저장 안 된 프로그램이 있어도 끄기");
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary(ui, "시작").clicked() {
                        if let Some(secs) = secs {
                            self.pc = Some(PcRun {
                                action: self.pc_action,
                                force: self.pc_force && self.pc_action.can_force(),
                                deadline: SystemTime::now() + Duration::from_secs(secs),
                                warning: false,
                            });
                            self.pc_msg = None;
                        }
                    }
                });
            });
        }
        ui.label(RichText::new(format!("{WARN_SECS}초 전에 경고 창이 떠요. 타이머가 도는 동안은 PC가 저절로 절전에 들어가지 않아요.")).weak().small());

        msg_line(ui, &mut self.pc_msg);
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
        let (mut cancel, mut snooze) = (false, false);
        egui::Modal::new(egui::Id::unique("pc-warning")).show(ctx, |ui| {
            ui.set_width(300.0);
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);
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
                run.deadline = SystemTime::now() + Duration::from_secs(SNOOZE_SECS);
                run.warning = false;
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
