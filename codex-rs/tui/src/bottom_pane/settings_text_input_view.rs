use codex_protocol::user_input::TextElement;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::CancellationEvent;
use crate::bottom_pane::ChatComposer;
use crate::bottom_pane::ChatComposerConfig;
use crate::bottom_pane::InputResult;
use crate::bottom_pane::ViewCompletion;
use crate::bottom_pane::bottom_pane_view::BottomPaneView;
use crate::bottom_pane::selection_popup_common::menu_surface_inset;
use crate::bottom_pane::selection_popup_common::menu_surface_padding_height;
use crate::bottom_pane::selection_popup_common::render_menu_surface;
use crate::keymap::RuntimeKeymap;
use crate::render::renderable::Renderable;
use crate::tui::FrameRequester;

const MIN_COMPOSER_HEIGHT: u16 = 3;
const MIN_VIEW_HEIGHT: u16 = 7;

pub(crate) type SettingsTextSubmitted = Box<dyn Fn(String) + Send + Sync>;
pub(crate) type SettingsTextCancelled = Box<dyn Fn() + Send + Sync>;

pub(crate) struct SettingsTextInputViewParams {
    pub(crate) title: String,
    pub(crate) placeholder: String,
    pub(crate) initial_text: String,
    pub(crate) secret: bool,
    pub(crate) on_submit: SettingsTextSubmitted,
    pub(crate) on_cancel: Option<SettingsTextCancelled>,
}

pub(crate) struct SettingsTextInputView {
    title: String,
    secret: bool,
    composer: ChatComposer,
    on_submit: SettingsTextSubmitted,
    on_cancel: Option<SettingsTextCancelled>,
    completion: Option<ViewCompletion>,
    validation_error: Option<String>,
}

impl SettingsTextInputView {
    pub(crate) fn new(
        params: SettingsTextInputViewParams,
        app_event_tx: AppEventSender,
        frame_requester: FrameRequester,
        keymap: &RuntimeKeymap,
        has_input_focus: bool,
        enhanced_keys_supported: bool,
        disable_paste_burst: bool,
    ) -> Self {
        let mut composer = ChatComposer::new_with_config(
            has_input_focus,
            app_event_tx,
            enhanced_keys_supported,
            params.placeholder,
            disable_paste_burst,
            ChatComposerConfig::plain_text(),
        );
        composer.set_frame_requester(frame_requester);
        composer.set_keymap_bindings(keymap);
        composer.set_footer_hint_override(Some(vec![
            ("Enter".to_string(), "save".to_string()),
            ("Esc".to_string(), "back".to_string()),
        ]));
        if !params.initial_text.is_empty() {
            composer.set_text_content(params.initial_text, Vec::<TextElement>::new(), Vec::new());
            composer.move_cursor_to_end();
        }

        Self {
            title: params.title,
            secret: params.secret,
            composer,
            on_submit: params.on_submit,
            on_cancel: params.on_cancel,
            completion: None,
            validation_error: None,
        }
    }

    fn submit(&mut self, text: String) {
        let text = text.trim().to_string();
        if text.is_empty() {
            self.validation_error = Some("Value required".to_string());
            return;
        }
        (self.on_submit)(text);
        self.completion = Some(ViewCompletion::Accepted);
    }
}

impl BottomPaneView for SettingsTextInputView {
    fn prefer_esc_to_handle_key_event(&self) -> bool {
        true
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.kind == KeyEventKind::Release {
            return;
        }
        if matches!(key_event.code, KeyCode::Esc) {
            if let Some(on_cancel) = &self.on_cancel {
                on_cancel();
            }
            self.completion = Some(ViewCompletion::Cancelled);
            return;
        }
        let (result, _) = self.composer.handle_key_event(key_event);
        if let InputResult::Submitted { text, .. } | InputResult::Queued { text, .. } = result {
            self.submit(text);
        }
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        if let Some(on_cancel) = &self.on_cancel {
            on_cancel();
        }
        self.completion = Some(ViewCompletion::Cancelled);
        CancellationEvent::Handled
    }

    fn is_complete(&self) -> bool {
        self.completion.is_some()
    }

    fn completion(&self) -> Option<ViewCompletion> {
        self.completion
    }

    fn handle_paste(&mut self, pasted: String) -> bool {
        self.composer.handle_paste(pasted)
    }

    fn flush_paste_burst_if_due(&mut self) -> bool {
        self.composer.flush_paste_burst_if_due()
    }

    fn is_in_paste_burst(&self) -> bool {
        self.composer.is_in_paste_burst()
    }
}

impl Renderable for SettingsTextInputView {
    fn desired_height(&self, width: u16) -> u16 {
        let inner = menu_surface_inset(Rect::new(/*x*/ 0, /*y*/ 0, width, u16::MAX));
        let input_height = self
            .composer
            .desired_height(inner.width.max(/*other*/ 1))
            .max(MIN_COMPOSER_HEIGHT);
        let error_height = u16::from(self.validation_error.is_some());
        1u16.saturating_add(input_height)
            .saturating_add(error_height)
            .saturating_add(menu_surface_padding_height())
            .max(MIN_VIEW_HEIGHT)
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let content = render_menu_surface(area, buf);
        if content.width == 0 || content.height == 0 {
            return;
        }
        let title_area = Rect {
            x: content.x,
            y: content.y,
            width: content.width,
            height: 1,
        };
        Paragraph::new(Line::from(self.title.clone()).bold()).render(title_area, buf);

        let error_height = u16::from(self.validation_error.is_some());
        let input_y = content.y.saturating_add(/*rhs*/ 1);
        let input_height = content
            .height
            .saturating_sub(/*rhs*/ 1)
            .saturating_sub(error_height);
        let input_area = Rect {
            x: content.x,
            y: input_y,
            width: content.width,
            height: input_height,
        };
        if self.secret {
            self.composer.render_with_mask(input_area, buf, Some('*'));
        } else {
            self.composer.render(input_area, buf);
        }

        if let Some(error) = &self.validation_error {
            let error_area = Rect {
                x: content.x,
                y: content.y + content.height.saturating_sub(/*rhs*/ 1),
                width: content.width,
                height: 1,
            };
            Paragraph::new(Line::from(error.clone()).red()).render(error_area, buf);
        }
    }
}
