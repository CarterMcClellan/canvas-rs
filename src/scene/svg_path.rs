//! SVG path string parser
//!
//! Parses SVG path `d` attribute strings into PathCommand vectors.
//! Supports all standard SVG path commands:
//! - M/m: moveto
//! - L/l: lineto
//! - H/h: horizontal lineto
//! - V/v: vertical lineto
//! - C/c: cubic bezier
//! - S/s: smooth cubic bezier
//! - Q/q: quadratic bezier
//! - T/t: smooth quadratic bezier
//! - A/a: elliptical arc
//! - Z/z: close path

use super::types::Vec2;
use super::PathCommand;

/// Parse an SVG path string into a vector of PathCommands
pub fn parse_svg_path(d: &str) -> Vec<PathCommand> {
    let mut commands = Vec::new();
    let mut tokenizer = PathTokenizer::new(d);

    let mut current_pos = Vec2::ZERO;
    let mut start_pos = Vec2::ZERO;
    let mut last_control: Option<Vec2> = None;
    let mut last_command: Option<char> = None;

    while let Some(cmd) = tokenizer.next_command() {
        let is_relative = cmd.is_ascii_lowercase();
        let cmd_upper = cmd.to_ascii_uppercase();

        match cmd_upper {
            'M' => {
                // MoveTo - first pair is moveto, subsequent pairs are lineto
                let mut first = true;
                while let Some((x, y)) = tokenizer.next_point() {
                    let point = if is_relative && !first {
                        Vec2::new(current_pos.x + x, current_pos.y + y)
                    } else if is_relative {
                        Vec2::new(current_pos.x + x, current_pos.y + y)
                    } else {
                        Vec2::new(x, y)
                    };

                    if first {
                        commands.push(PathCommand::MoveTo(point));
                        start_pos = point;
                        first = false;
                    } else {
                        commands.push(PathCommand::LineTo(point));
                    }
                    current_pos = point;
                }
                last_control = None;
                last_command = Some('M');
            }
            'L' => {
                while let Some((x, y)) = tokenizer.next_point() {
                    let point = if is_relative {
                        Vec2::new(current_pos.x + x, current_pos.y + y)
                    } else {
                        Vec2::new(x, y)
                    };
                    commands.push(PathCommand::LineTo(point));
                    current_pos = point;
                }
                last_control = None;
                last_command = Some('L');
            }
            'H' => {
                while let Some(x) = tokenizer.next_number() {
                    let new_x = if is_relative { current_pos.x + x } else { x };
                    let point = Vec2::new(new_x, current_pos.y);
                    commands.push(PathCommand::LineTo(point));
                    current_pos = point;
                }
                last_control = None;
                last_command = Some('H');
            }
            'V' => {
                while let Some(y) = tokenizer.next_number() {
                    let new_y = if is_relative { current_pos.y + y } else { y };
                    let point = Vec2::new(current_pos.x, new_y);
                    commands.push(PathCommand::LineTo(point));
                    current_pos = point;
                }
                last_control = None;
                last_command = Some('V');
            }
            'C' => {
                while let Some((x1, y1)) = tokenizer.next_point() {
                    let (x2, y2) = tokenizer.next_point().unwrap_or((x1, y1));
                    let (x, y) = tokenizer.next_point().unwrap_or((x2, y2));

                    let (ctrl1, ctrl2, end) = if is_relative {
                        (
                            Vec2::new(current_pos.x + x1, current_pos.y + y1),
                            Vec2::new(current_pos.x + x2, current_pos.y + y2),
                            Vec2::new(current_pos.x + x, current_pos.y + y),
                        )
                    } else {
                        (Vec2::new(x1, y1), Vec2::new(x2, y2), Vec2::new(x, y))
                    };

                    commands.push(PathCommand::CubicTo {
                        ctrl1,
                        ctrl2,
                        to: end,
                    });
                    last_control = Some(ctrl2);
                    current_pos = end;
                }
                last_command = Some('C');
            }
            'S' => {
                // Smooth cubic - first control point is reflection of last
                while let Some((x2, y2)) = tokenizer.next_point() {
                    let (x, y) = tokenizer.next_point().unwrap_or((x2, y2));

                    let ctrl1 = match (last_command, last_control) {
                        (Some('C'), Some(lc)) | (Some('S'), Some(lc)) => {
                            // Reflect last control point
                            Vec2::new(2.0 * current_pos.x - lc.x, 2.0 * current_pos.y - lc.y)
                        }
                        _ => current_pos,
                    };

                    let (ctrl2, end) = if is_relative {
                        (
                            Vec2::new(current_pos.x + x2, current_pos.y + y2),
                            Vec2::new(current_pos.x + x, current_pos.y + y),
                        )
                    } else {
                        (Vec2::new(x2, y2), Vec2::new(x, y))
                    };

                    commands.push(PathCommand::CubicTo {
                        ctrl1,
                        ctrl2,
                        to: end,
                    });
                    last_control = Some(ctrl2);
                    current_pos = end;
                }
                last_command = Some('S');
            }
            'Q' => {
                while let Some((x1, y1)) = tokenizer.next_point() {
                    let (x, y) = tokenizer.next_point().unwrap_or((x1, y1));

                    let (control, end) = if is_relative {
                        (
                            Vec2::new(current_pos.x + x1, current_pos.y + y1),
                            Vec2::new(current_pos.x + x, current_pos.y + y),
                        )
                    } else {
                        (Vec2::new(x1, y1), Vec2::new(x, y))
                    };

                    commands.push(PathCommand::QuadraticTo { control, to: end });
                    last_control = Some(control);
                    current_pos = end;
                }
                last_command = Some('Q');
            }
            'T' => {
                // Smooth quadratic - control point is reflection of last
                while let Some((x, y)) = tokenizer.next_point() {
                    let control = match (last_command, last_control) {
                        (Some('Q'), Some(lc)) | (Some('T'), Some(lc)) => {
                            Vec2::new(2.0 * current_pos.x - lc.x, 2.0 * current_pos.y - lc.y)
                        }
                        _ => current_pos,
                    };

                    let end = if is_relative {
                        Vec2::new(current_pos.x + x, current_pos.y + y)
                    } else {
                        Vec2::new(x, y)
                    };

                    commands.push(PathCommand::QuadraticTo { control, to: end });
                    last_control = Some(control);
                    current_pos = end;
                }
                last_command = Some('T');
            }
            'A' => {
                while let Some(arc) = tokenizer.next_arc() {
                    let end = if is_relative {
                        Vec2::new(current_pos.x + arc.x, current_pos.y + arc.y)
                    } else {
                        Vec2::new(arc.x, arc.y)
                    };

                    commands.push(PathCommand::ArcTo {
                        rx: arc.rx,
                        ry: arc.ry,
                        x_rotation: arc.x_rotation,
                        large_arc: arc.large_arc,
                        sweep: arc.sweep,
                        to: end,
                    });
                    current_pos = end;
                }
                last_control = None;
                last_command = Some('A');
            }
            'Z' => {
                commands.push(PathCommand::Close);
                current_pos = start_pos;
                last_control = None;
                last_command = Some('Z');
            }
            _ => {}
        }
    }

    commands
}

/// Arc parameters from SVG
struct ArcParams {
    rx: f32,
    ry: f32,
    x_rotation: f32,
    large_arc: bool,
    sweep: bool,
    x: f32,
    y: f32,
}

/// Simple tokenizer for SVG path strings
struct PathTokenizer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> PathTokenizer<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            chars: s.chars().peekable(),
        }
    }

    fn skip_whitespace_and_comma(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() || c == ',' {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn next_command(&mut self) -> Option<char> {
        self.skip_whitespace_and_comma();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphabetic() {
                self.chars.next();
                return Some(c);
            } else if c.is_whitespace() || c == ',' {
                self.chars.next();
            } else {
                break;
            }
        }
        None
    }

    fn peek_is_command(&mut self) -> bool {
        self.skip_whitespace_and_comma();
        if let Some(&c) = self.chars.peek() {
            c.is_alphabetic()
        } else {
            false
        }
    }

    fn next_number(&mut self) -> Option<f32> {
        self.skip_whitespace_and_comma();

        let mut s = String::new();

        // Handle sign
        if let Some(&c) = self.chars.peek() {
            if c == '-' || c == '+' {
                s.push(c);
                self.chars.next();
            }
        }

        // Handle digits before decimal
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.chars.next();
            } else {
                break;
            }
        }

        // Handle decimal point
        if let Some(&c) = self.chars.peek() {
            if c == '.' {
                s.push(c);
                self.chars.next();

                // Handle digits after decimal
                while let Some(&c) = self.chars.peek() {
                    if c.is_ascii_digit() {
                        s.push(c);
                        self.chars.next();
                    } else {
                        break;
                    }
                }
            }
        }

        // Handle exponent
        if let Some(&c) = self.chars.peek() {
            if c == 'e' || c == 'E' {
                s.push(c);
                self.chars.next();

                if let Some(&c) = self.chars.peek() {
                    if c == '-' || c == '+' {
                        s.push(c);
                        self.chars.next();
                    }
                }

                while let Some(&c) = self.chars.peek() {
                    if c.is_ascii_digit() {
                        s.push(c);
                        self.chars.next();
                    } else {
                        break;
                    }
                }
            }
        }

        if s.is_empty() || s == "-" || s == "+" {
            None
        } else {
            s.parse().ok()
        }
    }

    fn next_point(&mut self) -> Option<(f32, f32)> {
        if self.peek_is_command() {
            return None;
        }
        let x = self.next_number()?;
        let y = self.next_number()?;
        Some((x, y))
    }

    fn next_flag(&mut self) -> Option<bool> {
        self.skip_whitespace_and_comma();
        if let Some(&c) = self.chars.peek() {
            if c == '0' {
                self.chars.next();
                return Some(false);
            } else if c == '1' {
                self.chars.next();
                return Some(true);
            }
        }
        None
    }

    fn next_arc(&mut self) -> Option<ArcParams> {
        if self.peek_is_command() {
            return None;
        }
        let rx = self.next_number()?;
        let ry = self.next_number()?;
        let x_rotation = self.next_number()?;
        let large_arc = self.next_flag()?;
        let sweep = self.next_flag()?;
        let x = self.next_number()?;
        let y = self.next_number()?;

        Some(ArcParams {
            rx,
            ry,
            x_rotation,
            large_arc,
            sweep,
            x,
            y,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_path() {
        let cmds = parse_svg_path("M10 20 L30 40 Z");
        assert_eq!(cmds.len(), 3);
        assert!(matches!(cmds[0], PathCommand::MoveTo(p) if p.x == 10.0 && p.y == 20.0));
        assert!(matches!(cmds[1], PathCommand::LineTo(p) if p.x == 30.0 && p.y == 40.0));
        assert!(matches!(cmds[2], PathCommand::Close));
    }

    #[test]
    fn test_parse_relative() {
        let cmds = parse_svg_path("M10 20 l20 20");
        assert_eq!(cmds.len(), 2);
        assert!(matches!(cmds[0], PathCommand::MoveTo(p) if p.x == 10.0 && p.y == 20.0));
        assert!(matches!(cmds[1], PathCommand::LineTo(p) if p.x == 30.0 && p.y == 40.0));
    }

    #[test]
    fn test_parse_cubic() {
        let cmds = parse_svg_path("M0 0 C10 20 30 40 50 60");
        assert_eq!(cmds.len(), 2);
        assert!(matches!(&cmds[1], PathCommand::CubicTo { ctrl1, ctrl2, to }
            if ctrl1.x == 10.0 && ctrl2.x == 30.0 && to.x == 50.0));
    }

    #[test]
    fn test_parse_arc() {
        let cmds = parse_svg_path("M0 0 A5 10 45 1 0 20 30");
        assert_eq!(cmds.len(), 2);
        assert!(matches!(&cmds[1], PathCommand::ArcTo { rx, ry, x_rotation, large_arc, sweep, to }
            if *rx == 5.0 && *ry == 10.0 && *x_rotation == 45.0 && *large_arc && !*sweep && to.x == 20.0));
    }

    #[test]
    fn test_parse_h_v() {
        let cmds = parse_svg_path("M10 10 H50 V30");
        assert_eq!(cmds.len(), 3);
        assert!(matches!(cmds[1], PathCommand::LineTo(p) if p.x == 50.0 && p.y == 10.0));
        assert!(matches!(cmds[2], PathCommand::LineTo(p) if p.x == 50.0 && p.y == 30.0));
    }
}
