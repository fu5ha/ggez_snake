// First we'll import the crates we need for our game;
// in this case that is just `ggez` and `rand`.
extern crate ggez;
extern crate rand;

// Next we need to actually `use` the pieces of ggez that we are going
// to need frequently.
use ggez::event::KeyCode;
use ggez::{event, graphics, Context, GameResult};

// We'll bring in some things from `std` to help us in the future.
use std::collections::LinkedList;
use std::time::{Duration, Instant};

// And finally bring the `Rng` trait into scope so that we can generate
// some random numbers later.
use rand::Rng;

// The first thing we want to do is set up some constants that will help us out later.

// Here we define the size of our game board in terms of how many grid
// cells it will take up. We choose to make a 30 x 20 game board.
const GRID_SIZE: (i16, i16) = (30, 20);
// Now we define the pixel size of each tile, which we make 32x32 pixels.
const GRID_CELL_SIZE: (i16, i16) = (32, 32);

// Next we define how large we want our actual window to be by multiplying
// the components of our grid size by its corresponding pixel size.
const SCREEN_SIZE: (u32, u32) = (
    GRID_SIZE.0 as u32 * GRID_CELL_SIZE.0 as u32,
    GRID_SIZE.1 as u32 * GRID_CELL_SIZE.1 as u32,
);

// Here we're defining how many quickly we want our game to update. This will be
// important later so that we don't have our snake fly across the screen because
// it's moving a full tile every frame.
const UPDATES_PER_SECOND: f32 = 8.0;
// And we get the milliseconds of delay that this update rate corresponds to.
const MILLIS_PER_UPDATE: u64 = (1.0 / UPDATES_PER_SECOND * 1000.0) as u64;

// Now we define a struct that will hold an entity's position on our game board
// or grid which we defined above. We'll use signed integers because we only want
// to store whole numbers, and we need them to be signed so that they work properly
// with our modulus arithmetic later.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct GridPosition {
    x: i16,
    y: i16,
}

// This is a trait that provides a modulus function that works for negative values
// rather than just the standard remainder op (%) which does not. We'll use this
// to get our snake to wrap from one side of the game board around to the other
// when it goes off the top, bottom, left, or right side of the screen.
trait ModuloSigned {
    fn modulo(&self, n: Self) -> Self;
}

// Here we implement our `ModuloSigned` trait for any type T which implements
// `Add` (the `+` operator) with an output type T and Rem (the `%` operator)
// that also has anout put type of T, and that can be cloned. These are the bounds
// that we need in order to implement a modulus function that works for negative numbers
// as well.
impl<T> ModuloSigned for T
where
    T: std::ops::Add<Output = T> + std::ops::Rem<Output = T> + Clone,
{
    fn modulo(&self, n: T) -> T {
        // Because of our trait bounds, we can now apply these operators.
        (self.clone() % n.clone() + n.clone()) % n
    }
}

impl GridPosition {
    // We make a standard helper function so that we can create a new `GridPosition`
    // more easily.
    pub fn new(x: i16, y: i16) -> Self {
        GridPosition { x, y }
    }

    // As well as a helper function that will give us a random `GridPosition` from
    // `(0, 0)` to `(max_x, max_y)`
    pub fn random(max_x: i16, max_y: i16) -> Self {
        let mut rng = rand::thread_rng();
        // We can use `.into()` to convert from `(i16, i16)` to a `GridPosition` since
        // we implement `From<(i16, i16)>` for `GridPosition` below.
        (
            rng.gen_range::<i16>(0, max_x),
            rng.gen_range::<i16>(0, max_y),
        )
            .into()
    }

    // We'll make another helper function that takes one grid position and returns a new one after
    // making one move in the direction of `dir`. We use our `SignedModulo` trait
    // above, which is now implemented on `i16` because it satisfies the trait bounds,
    // to automatically wrap around within our grid size if the move would have otherwise
    // moved us off the board to the top, bottom, left, or right.
    pub fn new_from_move(pos: GridPosition, dir: Direction) -> Self {
        match dir {
            Direction::Up => GridPosition::new(pos.x, (pos.y - 1).modulo(GRID_SIZE.1)),
            Direction::Down => GridPosition::new(pos.x, (pos.y + 1).modulo(GRID_SIZE.1)),
            Direction::Left => GridPosition::new((pos.x - 1).modulo(GRID_SIZE.0), pos.y),
            Direction::Right => GridPosition::new((pos.x + 1).modulo(GRID_SIZE.0), pos.y),
        }
    }
}

// We implement the `From` trait, which in this case allows us to convert easily between
// a GridPosition and a ggez `graphics::Rect` which fills that grid cell.
// Now we can just call `.into()` on a `GridPosition` where we want a
// `Rect` that represents that grid cell.
impl From<GridPosition> for graphics::Rect {
    fn from(pos: GridPosition) -> Self {
        graphics::Rect::new_i32(
            pos.x as i32 * GRID_CELL_SIZE.0 as i32,
            pos.y as i32 * GRID_CELL_SIZE.1 as i32,
            GRID_CELL_SIZE.0 as i32,
            GRID_CELL_SIZE.1 as i32,
        )
    }
}

// And here we implement `From` again to allow us to easily convert between
// `(i16, i16)` and a `GridPosition`.
impl From<(i16, i16)> for GridPosition {
    fn from(pos: (i16, i16)) -> Self {
        GridPosition { x: pos.0, y: pos.1 }
    }
}

// Next we create an enum that will represent all the possible
// directions that our snake could move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    // We create a helper function that will allow us to easily get the inverse
    // of a `Direction` which we can use later to check if the player should be
    // able to move the snake in a certain direction.
    pub fn inverse(&self) -> Self {
        match *self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    // We also create a helper function that will let us convert between a
    // `ggez` `KeyCode` and the `Direction` that it represents. Of course,
    // not every keycode represents a direction, so we return `None` if this
    // is the case.
    pub fn from_keycode(key: KeyCode) -> Option<Direction> {
        match key {
            KeyCode::Up => Some(Direction::Up),
            KeyCode::Down => Some(Direction::Down),
            KeyCode::Left => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            _ => None,
        }
    }
}

// This is mostly just a semantic abstraction over a `GridPosition` to represent
// a segment of the snake. It could be useful to, say, have each segment contain its
// own color or something similar. This is an exercise left up to the reader ;)
#[derive(Clone, Copy, Debug)]
struct Segment {
    pos: GridPosition,
}

impl Segment {
    pub fn new(pos: GridPosition) -> Self {
        Segment { pos }
    }
}

// This is again an abstraction over a `GridPosition` that represents
// a piece of food the snake can eat. It can draw itself.
struct Food {
    pos: GridPosition,
}

impl Food {
    pub fn new(pos: GridPosition) -> Self {
        Food { pos }
    }

    // Here is the first time we see what drawing looks like with ggez.
    // We have a function that takes in a `&mut ggez::Context` which we use
    // with the helpers in `ggez::graphics` to do drawing. We also return a
    // `ggez::GameResult` so that we can use the `?` operator to bubble up
    // failure of drawing.
    fn draw(&self, ctx: &mut Context) -> GameResult {
        // First we have to create a MeshBuilder
        let mesh = graphics::MeshBuilder::new()
            // We call rectangle to make a square
            .rectangle(
                // Then we draw a rectangle with the Fill draw mode, and we convert the
                graphics::DrawMode::fill(),
                // since we implemented `From<GridPosition>` for `Rect` earlier.
                // Food's position into a `ggez::Rect` using `.into()` which we can do
                self.pos.into(),
                // Last we set the color to draw with, in this case all food will be
                // colored blue.
                graphics::Color::new(0.0, 0.0, 1.0, 1.0),
            )?
            .build(ctx)?;

        graphics::draw(ctx, &mesh, graphics::DrawParam::default())?;
        Ok(())
    }
}

// Here we define an enum of the possible things that the snake could have "eaten"
// during an update of the game. It could have either eaten a piece of `Food`, or
// it could have eaten `Itself` if the head ran into its body.
#[derive(Clone, Copy, Debug)]
enum Ate {
    Itself,
    Food,
}

// Now we make a struct that contains all the information needed to describe the
// state of the Snake itself.
struct Snake {
    // First we have the head of the snake, which is a single `Segment`.
    head: Segment,
    // Then we have the current direction the snake is moving. This is
    // the direction it will move when `update` is called on it.
    dir: Direction,
    // Next we have the body, which we choose to represent as a `LinkedList`
    // of `Segment`s.
    body: LinkedList<Segment>,
    // Now we have a property that represents the result of the last update
    // that was performed. The snake could have eaten nothing (None), Food (Some(Ate::Food)),
    // or Itself (Some(Ate::Itself))
    ate: Option<Ate>,
    // Finally we store the direction that the snake was traveling the last
    // time that `update` was called, which we will use to determine valid
    // directions that it could move the next time update is called.
    last_update_dir: Direction,
}

impl Snake {
    pub fn new(pos: GridPosition) -> Self {
        let mut body = LinkedList::new();
        // Our snake will initially have a head and one body segment,
        // and will be moving to the right.
        body.push_back(Segment::new((pos.x - 1, pos.y).into()));
        Snake {
            head: Segment::new(pos),
            dir: Direction::Right,
            last_update_dir: Direction::Right,
            body,
            ate: None,
        }
    }

    // A helper function that determines whether
    // the snake eats a given piece of Food based
    // on its current position
    fn eats(&self, food: &Food) -> bool {
        self.head.pos == food.pos
    }

    // A helper function that determines whether
    // the snake eats itself based on its current position
    fn eats_self(&self) -> bool {
        for seg in self.body.iter() {
            if self.head.pos == seg.pos {
                return true;
            }
        }
        false
    }

    // The main update function for our snake which gets called every time
    // we want to update the game state.
    fn update(&mut self, food: &Food) {
        // First we get a new head position by using our `new_from_move` helper
        // function from earlier. We move our head in the direction we are currently
        // heading.
        let new_head_pos = GridPosition::new_from_move(self.head.pos, self.dir);
        // Next we create a new segment will be our new head segment using the
        // new position we just made.
        let new_head = Segment::new(new_head_pos);
        // Then we push our current head Segment onto the front of our body
        self.body.push_front(self.head);
        // And finally make our actual head the new Segment we created. This has
        // effectively moved the snake in the current direction.
        self.head = new_head;
        // Next we check whether the snake eats itself or some food, and if so,
        // we set our `ate` member to reflect that state.
        if self.eats_self() {
            self.ate = Some(Ate::Itself);
        } else if self.eats(food) {
            self.ate = Some(Ate::Food);
        } else {
            self.ate = None
        }
        // If we didn't eat anything this turn, we remove the last segment from our body,
        // which gives the illusion that the snake is moving. In reality, all the segments stay
        // stationary, we just add a segment to the front and remove one from the back. If we eat
        // a piece of food, then we leave the last segment so that we extend our body by one.
        if self.ate.is_none() {
            self.body.pop_back();
        }
        // And set our last_update_dir to the direction we just moved.
        self.last_update_dir = self.dir;
    }

    // Here we have the Snake draw itself. This is very similar to how we saw the Food
    // draw itself earlier.
    fn draw(&self, ctx: &mut Context) -> GameResult {
        // We first iterate through the body segments and draw them.
        for seg in self.body.iter() {
            // First we create a new MeshBuilder
            let mesh = graphics::MeshBuilder::new()
                // Since we want a square we call rectangle method
                .rectangle(
                    // Then set DrawMode to fill the rectangle
                    graphics::DrawMode::fill(),
                    // We use `.into` (provided by the rust `Into` trait) to convert our position to the `mint` type that ggez's api wants
                    seg.pos.into(),
                    // Again we set the color (in this case an orangey color)
                    graphics::Color::new(1.0, 0.5, 0.0, 1.0),
                )?
                .build(ctx)?;
            graphics::draw(ctx, &mesh, graphics::DrawParam::default())?;
        }
        // And then we do the same for the head, instead making it fully red to distinguish it.

        // And then we do the same for the head, instead making it fully red to distinguish it
        let mesh = graphics::MeshBuilder::new()
            .rectangle(
                graphics::DrawMode::fill(),
                self.head.pos.into(),
                graphics::Color::new(1.0, 0.0, 0.0, 1.0),
            )?
            .build(ctx)?;

        graphics::draw(ctx, &mesh, graphics::DrawParam::default())?;
        Ok(())
    }
}

// Now we have the heart of our game, the GameState. This struct
// will implement ggez's `EventHandler` trait and will therefore drive
// everything else that happens in our game.
struct GameState {
    // First we need a Snake
    snake: Snake,
    // A piee of food
    food: Food,
    // Whether the game is over or not
    gameover: bool,
    // And we track the last time we updated so that we can limit
    // our update rate.
    last_update: Instant,
}

impl GameState {
    // Our new function will set up the initial state of our game.
    pub fn new() -> GameResult<Self> {
        // First we put our snake a quarter of the way across our grid in the x axis
        // and half way down the y axis. This works well since we start out moving to the right.
        let snake_pos = (GRID_SIZE.0 / 4, GRID_SIZE.1 / 2).into();
        // Then we choose a random place to put our piece of food using the helper we made
        // earlier.
        let food_pos = GridPosition::random(GRID_SIZE.0, GRID_SIZE.1);

        Ok(GameState {
            snake: Snake::new(snake_pos),
            food: Food::new(food_pos),
            gameover: false,
            last_update: Instant::now(),
        })
    }
}

// Now we implement EventHandler for GameState. This provides an interface
// that ggez will call automatically when different events happen.
impl event::EventHandler<ggez::GameError> for GameState {
    // Update will happen on every frame before it is drawn. This is where we update
    // our game state to react to whatever is happening in the game world.
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        // First we check to see if enough time has elapsed since our last update based on
        // the update rate we defined at the top.
        if Instant::now() - self.last_update >= Duration::from_millis(MILLIS_PER_UPDATE) {
            // Then we check to see if the game is over. If not, we'll update. If so, we'll just do nothing.
            if !self.gameover {
                // Here we do the actual updating of our game world. First we tell the snake to update itself,
                // passing in a reference to our piece of food.
                self.snake.update(&self.food);
                // Next we check if the snake ate anything as it updated.
                if let Some(ate) = self.snake.ate {
                    // If it did, we want to know what it ate.
                    match ate {
                        // If it ate a piece of food, we randomly select a new position for our piece of food
                        // and move it to this new position.
                        Ate::Food => {
                            let new_food_pos = GridPosition::random(GRID_SIZE.0, GRID_SIZE.1);
                            self.food.pos = new_food_pos;
                        }
                        // If it ate itself, we set our gameover state to true.
                        Ate::Itself => {
                            self.gameover = true;
                        }
                    }
                }
            }
            // If we updated, we set our last_update to be now
            self.last_update = Instant::now();
        }
        // Finally we return `Ok` to indicate we didn't run into any errors
        Ok(())
    }

    // draw is where we should actually render the game's current state.
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // First we clear the screen and
        // We set the background color to a nice (well, maybe pretty glaring ;)) green
        graphics::clear(ctx, [0.0, 1.0, 0.0, 1.0].into());
        // Then we tell the snake and the food to draw themselves
        self.snake.draw(ctx)?;
        self.food.draw(ctx)?;
        // Finally we call graphics::present to cycle the gpu's framebuffer and display
        // the new frame we just drew.
        graphics::present(ctx)?;
        // We yield the current thread until the next update
        ggez::timer::yield_now();
        // And return success.
        Ok(())
    }

    // key_down_event gets fired when a key gets pressed.
    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymod: ggez::input::keyboard::KeyMods,
        _repeat: bool,
    ) {
        // Here we attempt to convert the KeyCode into a Direction using the helper
        // we defined earlier.
        if let Some(dir) = Direction::from_keycode(keycode) {
            // If it succeeds, we check to make sure that the direction being pressed
            // is not directly opposite to the way the snake was facing last update.
            if dir.inverse() != self.snake.last_update_dir {
                // If not, we set the snake's new direction to be the direction the user pressed.
                self.snake.dir = dir;
            }
        }
    }
}

fn main() -> GameResult {
    // Here we use a ContextBuilder to setup metadata about our game. First the title and author
    let (ctx, event_loop) = ggez::ContextBuilder::new("snake", "Gray Olson")
        // Next we set up the window. This title will be displayed in the title bar of the window.
        .window_setup(ggez::conf::WindowSetup::default().title("Snake!"))
        // Now we get to set the size of the window, which we use our SCREEN_SIZE constant from earlier to help with
        .window_mode(
            ggez::conf::WindowMode::default()
                .dimensions(SCREEN_SIZE.0 as f32, SCREEN_SIZE.1 as f32),
        )
        // And finally we attempt to build the context and create the window. If it fails, we panic with the message
        // "Failed to build ggez context"
        .build()
        .expect("Failed to build ggez context");

    // Next we create a new instance of our GameState struct, which implements EventHandler
    let state = GameState::new()?;
    // And finally we actually run our game, passing in our context, event_loop and state.
    event::run(ctx, event_loop, state)
}
