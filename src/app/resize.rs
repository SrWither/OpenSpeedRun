use eframe::egui::{Area, Context, CursorIcon, Id, Order, Rect, Sense, Vec2, ViewportCommand};

#[derive(Debug, Clone, Copy)]
enum ResizeEdge {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

pub fn draw_resize_borders(ctx: &Context) {
    let margin = 8.0;
    let rect = ctx.screen_rect();

    Area::new("resize_layer".into())
        .order(Order::Foreground)
        .interactable(false)
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            let zones: [(ResizeEdge, Rect); 8] = [
                (
                    ResizeEdge::North,
                    Rect::from_min_max(rect.left_top(), rect.right_top() + Vec2::new(0.0, margin)),
                ),
                (
                    ResizeEdge::South,
                    Rect::from_min_max(
                        rect.left_bottom() - Vec2::new(0.0, margin),
                        rect.right_bottom(),
                    ),
                ),
                (
                    ResizeEdge::West,
                    Rect::from_min_max(
                        rect.left_top(),
                        rect.left_bottom() + Vec2::new(margin, 0.0),
                    ),
                ),
                (
                    ResizeEdge::East,
                    Rect::from_min_max(
                        rect.right_top() - Vec2::new(margin, 0.0),
                        rect.right_bottom(),
                    ),
                ),
                (
                    ResizeEdge::NorthWest,
                    Rect::from_min_max(rect.left_top(), rect.left_top() + Vec2::splat(margin)),
                ),
                (
                    ResizeEdge::NorthEast,
                    Rect::from_min_max(
                        rect.right_top() - Vec2::new(margin, 0.0),
                        rect.right_top() + Vec2::new(0.0, margin),
                    ),
                ),
                (
                    ResizeEdge::SouthWest,
                    Rect::from_min_max(
                        rect.left_bottom() - Vec2::new(0.0, margin),
                        rect.left_bottom() + Vec2::new(margin, 0.0),
                    ),
                ),
                (
                    ResizeEdge::SouthEast,
                    Rect::from_min_max(
                        rect.right_bottom() - Vec2::splat(margin),
                        rect.right_bottom(),
                    ),
                ),
            ];

            for (edge, zone) in zones {
                let id = Id::new(format!("resize_zone_{edge:?}"));
                let response = ui.interact(zone, id, Sense::click_and_drag());

                if response.hovered() {
                    ctx.output_mut(|o| {
                        o.cursor_icon = match edge {
                            ResizeEdge::North => CursorIcon::ResizeNorth,
                            ResizeEdge::South => CursorIcon::ResizeSouth,
                            ResizeEdge::East => CursorIcon::ResizeEast,
                            ResizeEdge::West => CursorIcon::ResizeWest,
                            ResizeEdge::NorthEast => CursorIcon::ResizeNeSw,
                            ResizeEdge::NorthWest => CursorIcon::ResizeNwSe,
                            ResizeEdge::SouthEast => CursorIcon::ResizeNwSe,
                            ResizeEdge::SouthWest => CursorIcon::ResizeNeSw,
                        }
                    });
                }

                if response.dragged() {
                    let delta = response.drag_delta();
                    let mut new_size = rect.size();
                    let mut new_pos = rect.min;

                    match edge {
                        ResizeEdge::North => {
                            new_size.y -= delta.y;
                            new_pos.y += delta.y;
                        }
                        ResizeEdge::South => {
                            new_size.y += delta.y;
                        }
                        ResizeEdge::East => {
                            new_size.x += delta.x;
                        }
                        ResizeEdge::West => {
                            new_size.x -= delta.x;
                            new_pos.x += delta.x;
                        }
                        ResizeEdge::NorthWest => {
                            new_size -= delta;
                            new_pos += delta;
                        }
                        ResizeEdge::NorthEast => {
                            new_size.x += delta.x;
                            new_size.y -= delta.y;
                            new_pos.y += delta.y;
                        }
                        ResizeEdge::SouthWest => {
                            new_size.x -= delta.x;
                            new_size.y += delta.y;
                            new_pos.x += delta.x;
                        }
                        ResizeEdge::SouthEast => {
                            new_size += delta;
                        }
                    }

                    new_size.x = new_size.x.max(100.0);
                    new_size.y = new_size.y.max(100.0);

                    ctx.send_viewport_cmd(ViewportCommand::InnerSize(new_size));
                    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(new_pos));
                }
            }
        });
}
