use glam::Vec2;
use crate::engine::rendering::LocalTransform;
use crate::engine::rendering::text::{FontID, FontLineMetrics, Glyph, GlyphMetrics};
use crate::engine::world::World;

pub struct TextBlock {
    pub text: String,
    pub font: FontID,
    pub size_px: f32,
    pub glyphs: Vec<Glyph>,
    /// Max pixel width of a line before wrapping. None disables width-based
    /// wrapping, but text still breaks on \n.
    pub line_width_px: Option<f32>,
    /// Max pixel height of the whole text block. Kines beyond this are still
    /// laid out logically and count toward line_count, but get_local_transforms
    /// returns None for their glyphs, since there's nothing sensible to render for
    /// them. None disables the cap entirely.
    pub max_height_px: Option<f32>,
    line_metrics: FontLineMetrics,
    /// Glyph indices where each line begins. Always has the first entry as 0 even for empty text.
    line_starts: Vec<usize>,
}

impl TextBlock {
    pub fn new(world: &mut World, text: &str, font: FontID, size_px: f32) -> Result<TextBlock, String> {
        let mut s = Self {
            text: String::new(),
            font,
            size_px,
            glyphs: Vec::new(),
            line_width_px: None,
            max_height_px: None,
            line_metrics: FontLineMetrics {
                ascent: 0.0,
                descent: 0.0,
                line_gap: 0.0,
                new_line_size: size_px,
            },
            line_starts: vec![0],
        };

        s.set_text(world, text)?;

        Ok(s)
    }

    pub fn with_wrapping(mut self, line_width_px: Option<f32>, max_height_px: Option<f32>) -> Self {
        self.line_width_px = line_width_px;
        self.max_height_px = max_height_px;
        self.recompute_layout();
        self
    }

    pub fn set_line_width(mut self, line_width_px: Option<f32>) -> Self {
        self.line_width_px = line_width_px;
        self.recompute_layout();
        self
    }

    pub fn set_max_height(mut self, line_height_px: Option<f32>) -> Self {
        self.max_height_px = line_height_px;
        self.recompute_layout();
        self
    }

    pub fn set_text(&mut self, world: &mut World, text: &str) -> Result<u32, String> {
        let gl = world.gl();
        let text_manager = world.text_manager_mut();

        self.line_metrics = text_manager
            .font_line_metrics(self.font, self.size_px)
            .ok_or_else(|| {
                format!(
                    "No line metrics for font {:?} at size {}",
                    self.font, self.size_px
                )
            })?;

        let mut glyphs = Vec::with_capacity(text.chars().count());

        for ch in text.chars() {
            if ch == '\n' {
                glyphs.push(Glyph {
                    metrics: GlyphMetrics {
                        advance_width: 0.0,
                        bearing_x: 0.0,
                        bearing_y: 0.0,
                        width: 0.0,
                        height: 0.0,
                    },
                    visual: None,
                    ch
                });
                continue;
            }

            match text_manager.glyph(gl.clone(), self.font, self.size_px, ch) {
                Ok(glyph) => glyphs.push(glyph),
                Err(e) => {
                    return Err(format!(
                        "Glyph for character '{}' could not be made with font {} at size {}: {}",
                        ch, self.font, self.size_px, e
                    ))
                }
            }
        }

        self.text = text.to_string();
        self.glyphs = glyphs;
        self.recompute_layout();

        Ok(self.glyphs.len() as u32)
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// how many of the line_count lines actually fit in the current vertical cap
    pub fn visible_line_count(&self) -> usize {
        let Some(max_h) = self.max_height_px else {
            return self.line_count();
        };
        if self.line_metrics.new_line_size <= 0.0 {
            return self.line_count();
        }

        (0..self.line_count())
            .take_while(|&i| (i as f32) * self.line_metrics.new_line_size <= max_h)
            .count()
    }

    /// Distance from one line's baseline to the next. Useful for laying out objects
    /// along the text lines (like a caret, backgrounds, etc.)
    pub fn line_height(&self) -> f32 {
        self.line_metrics.new_line_size
    }

    /// Computes each glyph's LocalTransform given a starting pen position (the first line's
    /// baseline in the parent entity's local pixel space).
    ///
    /// Lines before start_line and after visible_line_count + start_line yield None for all their glyphs.
    /// The result is always the same length as self.glyphs, in the same order, so callers can zip them
    /// together directly when building RenderItems.
    pub fn get_local_transforms(&self, start_pen: Vec2, start_line: usize) -> Vec<Option<LocalTransform>> {
        let mut result = Vec::with_capacity(self.glyphs.len());

        let visible_lines = self.visible_line_count();
        let end_line = start_line + visible_lines;

        let mut line_idx = 0usize;
        let mut next_line_cursor = 1usize;

        let mut pen = start_pen;
        let mut current_visible_line = 0usize;

        for (i, glyph) in self.glyphs.iter().enumerate() {
            // Advance to the next line.
            if next_line_cursor < self.line_starts.len() && i == self.line_starts[next_line_cursor]
            {
                line_idx += 1;
                next_line_cursor += 1;

                // Only move the pen if this is a visible line.
                if line_idx >= start_line && line_idx < end_line {
                    current_visible_line += 1;
                    pen = Vec2::new(
                        start_pen.x,
                        start_pen.y - current_visible_line as f32 * self.line_metrics.new_line_size,
                    );
                }
            }

            if line_idx < start_line || line_idx >= end_line {
                result.push(None);
                continue;
            }

            result.push(glyph.quad_local_transform(pen));
            pen = glyph.advance(pen);
        }

        result
    }

    /// Recomputes line_starts from the current self.glyphs, self.text and self.line_width_px.
    fn recompute_layout(&mut self) {
        self.line_starts.clear();
        self.line_starts.push(0);

        if self.glyphs.is_empty() {
            return;
        }

        let chars: Vec<char> = self.text.chars().collect();
        debug_assert_eq!(chars.len(), self.glyphs.len());

        enum Token {
            Word { start: usize, width: f32 },
            Space { index: usize, width: f32 },
            NewLine { index: usize },
        }

        // Tokenize into words / spaces / explicit newlines, each carrying the total
        // pixel advance of the glyphs it covers.

        let mut tokens = Vec::new();
        let mut word_start: Option<usize> = None;
        let mut word_width = 0.0f32;

        for (i, &ch) in chars.iter().enumerate() {
            let advance = self.glyphs[i].metrics.advance_width;

            if ch == '\n' {
                if let Some(start) = word_start.take() {
                    tokens.push(Token::Word { start, width: word_width });
                    word_width = 0.0;
                }
                tokens.push(Token::NewLine { index: i });
            } else if ch == ' ' {
                if let Some(start) = word_start.take() {
                    tokens.push(Token::Word { start, width: word_width });
                    word_width = 0.0;
                }
                tokens.push(Token::Space { index: i, width: advance });
            } else {
                if word_start.is_none() {
                    word_start = Some(i);
                }
                word_width += advance;
            }
        }
        if let Some(start) = word_start.take() {
            tokens.push(Token::Word { start, width: word_width });
        }

        // Greedily pack tokens onto lines
        let mut cur_width = 0.0f32;

        for token in tokens {
            match token {
                Token::NewLine { index } => {
                    self.line_starts.push(index + 1);
                    cur_width = 0.0;
                },
                Token::Space { index, width } => {
                    if let Some(max_width) = self.line_width_px {
                        if cur_width > 0.0 && cur_width + width > max_width {
                            // Wrap here. Space collapses into break instead of becoming leading whitespace on the next line.
                            self.line_starts.push(index + 1);
                            cur_width = 0.0;
                            continue;
                        }
                    }
                    cur_width += width;
                },
                Token::Word { start, width } => {
                    if let Some(max_width) = self.line_width_px {
                        if cur_width > 0.0 && cur_width + width > max_width {
                            self.line_starts.push(start);
                            cur_width = 0.0;
                        }
                    }
                    cur_width += width;
                    // A single word wider than line_width_px on its own will overflow the line.
                }
            }
        }
    }
}