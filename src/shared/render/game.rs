use super::*;
use shared::state::MainState;
use shared::soldier::Soldier;
use shared::mapfile::MapFile;
use std::str::FromStr;
use ini::Ini;

pub struct GameGraphics {
    map: MapGraphics,
    gostek: GostekGraphics,
    sprites: Vec<Vec<Sprite>>,
    batch: DrawBatch,
}

impl GameGraphics {
    pub fn new(_context: &mut Gfx2dContext) -> GameGraphics {
        GameGraphics {
            map: MapGraphics::empty(),
            gostek: GostekGraphics::new(),
            sprites: Vec::new(),
            batch: DrawBatch::new(),
        }
    }

    pub fn render_frame(&mut self, context: &mut Gfx2dContext, state: &MainState, soldier: &Soldier,
        _elapsed: f64, frame_percent: f32)
    {
        let zoom = f32::exp(state.zoom);
        let cam = lerp(state.camera_prev, state.camera, frame_percent);
        let (w, h) = (zoom*state.game_width, zoom*state.game_height);
        let (dx, dy) = (cam.x - w / 2.0, cam.y - h / 2.0);
        let transform = Transform::ortho(dx, dx + w, dy, dy + h).matrix();

        self.batch.clear();
        self.gostek.render(soldier, &mut self.batch, &self.sprites, frame_percent);

        if false {
            let px = h / context.wnd.get_inner_size().unwrap().1 as f32;
            GostekGraphics::render_skeleton(soldier, &mut self.batch, px, frame_percent);
        }

        context.clear(rgb(0, 0, 0));
        context.draw(&mut self.map.background(), &Transform::ortho(0.0, 1.0, dy, dy + h).matrix());
        context.draw(&mut self.map.polys_back(), &transform);
        context.draw(&mut self.map.scenery_back(), &transform);
        context.draw(&mut self.batch.all(), &transform);
        context.draw(&mut self.map.scenery_mid(), &transform);
        context.draw(&mut self.map.polys_front(), &transform);
        context.draw(&mut self.map.scenery_front(), &transform);
        self.render_cursor(context, state);
    }

    fn render_cursor(&mut self, context: &mut Gfx2dContext, state: &MainState) {
        let zoom = f32::exp(state.zoom);
        let (w, h) = (zoom*state.game_width, zoom*state.game_height);
        let size = context.wnd.get_inner_size().unwrap();
        let size = vec2(size.0 as f32, size.1 as f32);
        let x = zoom * f32::floor(state.mouse.x * size.x / w);
        let y = zoom * f32::floor(state.mouse.y * size.y / h);
        let screen = Transform::ortho(0.0, size.x, 0.0, size.y).matrix();

        self.batch.clear();

        self.batch.add_quad(None, &[
            vertex(vec2(x, y) + vec2(0.0, -8.0), Vec2::zeros(), rgb(0, 0, 0)),
            vertex(vec2(x, y) + vec2(1.0, -8.0), Vec2::zeros(), rgb(0, 0, 0)),
            vertex(vec2(x, y) + vec2(1.0,  9.0), Vec2::zeros(), rgb(0, 0, 0)),
            vertex(vec2(x, y) + vec2(0.0,  9.0), Vec2::zeros(), rgb(0, 0, 0)),
        ]);

        self.batch.add_quad(None, &[
            vertex(vec2(x, y) + vec2(-8.0, 0.0), Vec2::zeros(), rgb(0, 0, 0)),
            vertex(vec2(x, y) + vec2( 9.0, 0.0), Vec2::zeros(), rgb(0, 0, 0)),
            vertex(vec2(x, y) + vec2( 9.0, 1.0), Vec2::zeros(), rgb(0, 0, 0)),
            vertex(vec2(x, y) + vec2(-8.0, 1.0), Vec2::zeros(), rgb(0, 0, 0)),
        ]);

        context.draw(&mut self.batch.all(), &screen);
    }

    pub fn load_map(&mut self, context: &mut Gfx2dContext, map: &MapFile) {
        self.map = MapGraphics::new(context, map);
    }

    pub fn load_sprites(&mut self, context: &mut Gfx2dContext) {
        let mut main: Vec<SpriteInfo> = Vec::new();
        let mut intf: Vec<SpriteInfo> = Vec::new();

        let add_to = |v: &mut Vec<SpriteInfo>, fname: &str| {
            let fname = filename_override("assets/", fname);
            v.push(SpriteInfo::new(fname, vec2(1.0, 1.0), None));
        };

        for group in SpriteGroup::values() {
            match *group {
                SpriteGroup::Gostek    => Gostek   ::values().iter().map(|v| v.filename()).for_each(|f| add_to(&mut main, f)),
                SpriteGroup::Weapon    => Weapon   ::values().iter().map(|v| v.filename()).for_each(|f| add_to(&mut main, f)),
                SpriteGroup::Spark     => Spark    ::values().iter().map(|v| v.filename()).for_each(|f| add_to(&mut main, f)),
                SpriteGroup::Object    => Object   ::values().iter().map(|v| v.filename()).for_each(|f| add_to(&mut main, f)),
                SpriteGroup::Interface => Interface::values().iter().map(|v| v.filename()).for_each(|f| add_to(&mut intf, f)),
            }
        }

        if let Ok(cfg) = Ini::load_from_file("assets/mod.ini") {
            self.gostek.load_data(&cfg);

            if let Some(data) = cfg.section(Some("SCALE".to_owned())) {
                let default_scale = match data.get("DefaultScale") {
                    None => 1.0,
                    Some(scale) => f32::from_str(scale).unwrap_or(1.0),
                };

                for sprite_info in main.iter_mut().chain(intf.iter_mut()) {
                    let fname = sprite_info.filename.strip_prefix("assets/").unwrap().to_str().unwrap();

                    let scale = match data.get(fname) {
                        None => default_scale,
                        Some(scale) => f32::from_str(scale).unwrap_or(default_scale),
                    };

                    sprite_info.pixel_ratio = vec2(scale, scale);
                }
            }
        }

        let main = Spritesheet::new(context, 8, FilterMethod::Trilinear, &main);
        let intf = Spritesheet::new(context, 8, FilterMethod::Trilinear, &intf);

        self.sprites.clear();
        self.sprites.resize(SpriteGroup::values().len(), Vec::new());

        let mut imain = 0;
        let mut iintf = 0;

        for group in SpriteGroup::values() {
            let index = group.id();

            match *group {
                SpriteGroup::Gostek => for _ in Gostek::values() {
                    self.sprites[index].push(main.sprites[imain].clone()); imain += 1;
                }
                SpriteGroup::Weapon => for _ in Weapon::values() {
                    self.sprites[index].push(main.sprites[imain].clone()); imain += 1;
                }
                SpriteGroup::Spark => for _ in Spark::values() {
                    self.sprites[index].push(main.sprites[imain].clone()); imain += 1;
                }
                SpriteGroup::Object => for _ in Object::values() {
                    self.sprites[index].push(main.sprites[imain].clone()); imain += 1;
                }
                SpriteGroup::Interface => for _ in Interface::values() {
                    self.sprites[index].push(intf.sprites[iintf].clone()); iintf += 1;
                }
            }
        }
    }
}
