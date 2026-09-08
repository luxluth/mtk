use mtk::colors::Color;
use mtk::effects::Radius;
use mtk::style::{
    AlignItems, Edges, FlexDirection, JustifyContent, PositionStrategy, ScrollbarStyle, Size,
    Style, TextStyle,
};
use mtk::text_property::FontWeight;
use mtk::ui::ViewStyleExt;
use mtk::ui::event::{ThumbScrollContext, ViewEventExt};
use mtk::ui::style::StyledView;
use mtk::ui::widgets::{Text, column, row, spacer, text, virtual_list};
use mtk::windowing::{Window, WindowAttributes};
use mtk::{clr, rgba};

#[derive(Clone)]
struct Contact {
    name: String,
    role: String,
    letter: char,
    color: Color,
    is_section_start: bool,
}

struct AppState {
    contacts: Vec<Contact>,
    is_scrubbing: bool,
    scrub_letter: char,
    scrub_thumb_y: f32,
    scroll_pct: f32,
}

#[derive(Clone, Debug)]
enum AppMsg {
    ThumbScrolled(ThumbScrollContext),
}

fn update(state: &mut AppState, msg: AppMsg) {
    match msg {
        AppMsg::ThumbScrolled(ctx) => {
            state.is_scrubbing = ctx.is_dragging;
            state.scroll_pct = ctx.scroll_pct;
            state.scrub_thumb_y = ctx.thumb.y;

            let letter_idx = ((ctx.scroll_pct * 25.99) as usize).clamp(0, 25);
            state.scrub_letter = (b'A' + letter_idx as u8) as char;
        }
    }
}

fn avatar_color_for_char(c: char) -> Color {
    let palette = [
        clr!(0xef4444ff), // red
        clr!(0xf97316ff), // orange
        clr!(0xf59e0bff), // amber
        clr!(0x10b981ff), // emerald
        clr!(0x06b6d4ff), // cyan
        clr!(0x0ea5e9ff), // sky
        clr!(0x3b82f6ff), // blue
        clr!(0x6366f1ff), // indigo
        clr!(0x8b5cf6ff), // violet
        clr!(0xa855f7ff), // purple
        clr!(0xd946efff), // fuchsia
        clr!(0xec4899ff), // pink
        clr!(0x14b8a6ff), // teal
        clr!(0x84cc16ff), // lime
        clr!(0x64748bff), // slate
        clr!(0x475569ff), // dark slate
    ];
    let idx = (c as usize) % palette.len();
    palette[idx]
}

fn generate_sample_contacts() -> Vec<Contact> {
    let first_names_by_letter: [(&str, &[&str]); 26] = [
        (
            "A",
            &[
                "Aaron",
                "Abigail",
                "Adam",
                "Adrian",
                "Aiden",
                "Alexander",
                "Alice",
                "Amelia",
                "Andrew",
                "Anna",
                "Anthony",
                "Archer",
                "Arthur",
                "Asher",
                "Ashley",
                "Audrey",
                "Austin",
                "Ava",
                "Avery",
                "Axel",
            ],
        ),
        (
            "B",
            &[
                "Bailey", "Barbara", "Beatrice", "Beau", "Beckett", "Bella", "Benjamin", "Bennett",
                "Bentley", "Blake", "Bradley", "Brady", "Brandon", "Braxton", "Brayden", "Brendan",
                "Brianna", "Brody", "Brooke", "Brooklyn",
            ],
        ),
        (
            "C",
            &[
                "Caden",
                "Caleb",
                "Callum",
                "Cameron",
                "Camila",
                "Carla",
                "Carlos",
                "Caroline",
                "Carter",
                "Catherine",
                "Cecilia",
                "Charles",
                "Charlotte",
                "Chase",
                "Chloe",
                "Christian",
                "Christopher",
                "Claire",
                "Cole",
                "Connor",
            ],
        ),
        (
            "D",
            &[
                "Daisy", "Dakota", "Dallas", "Damian", "Daniel", "Danielle", "Dante", "Daphne",
                "David", "Dawson", "Dean", "Declan", "Delilah", "Derek", "Diana", "Diego",
                "Dominic", "Donovan", "Douglas", "Dylan",
            ],
        ),
        (
            "E",
            &[
                "Easton",
                "Eden",
                "Edward",
                "Elena",
                "Eli",
                "Eliana",
                "Elias",
                "Elijah",
                "Elizabeth",
                "Ella",
                "Ellie",
                "Elliott",
                "Ellis",
                "Emerson",
                "Emery",
                "Emilia",
                "Emily",
                "Emma",
                "Ethan",
                "Everett",
            ],
        ),
        (
            "F",
            &[
                "Fabian", "Faith", "Farrah", "Felix", "Fernando", "Finley", "Finn", "Fiona",
                "Fletcher", "Flora", "Florence", "Flynn", "Foster", "Frances", "Francis", "Frank",
                "Franklin", "Freya", "Frida", "Fritz",
            ],
        ),
        (
            "G",
            &[
                "Gabriel",
                "Gabriella",
                "Gael",
                "Gage",
                "Garrett",
                "Gavin",
                "Genevieve",
                "George",
                "Georgia",
                "Gianna",
                "Gideon",
                "Gemma",
                "Gilbert",
                "Giovanni",
                "Giselle",
                "Grace",
                "Graham",
                "Grant",
                "Grayson",
                "Gregory",
            ],
        ),
        (
            "H",
            &[
                "Hadley", "Hailey", "Hannah", "Harley", "Harmony", "Harper", "Harrison", "Harvey",
                "Hayden", "Hayes", "Hazel", "Heath", "Hector", "Heidi", "Henry", "Holden", "Holly",
                "Hope", "Hudson", "Hunter",
            ],
        ),
        (
            "I",
            &[
                "Ian", "Ibrahim", "Ida", "Ignacio", "Iliana", "Imani", "Imogen", "Ingrid", "Ira",
                "Irene", "Iris", "Irvin", "Isaac", "Isabel", "Isabella", "Isaiah", "Isla", "Ivan",
                "Ivanna", "Ivy",
            ],
        ),
        (
            "J",
            &[
                "Jack", "Jackson", "Jacob", "Jaden", "James", "Jasmine", "Jason", "Jasper",
                "Jaxon", "Jayden", "Jenna", "Jennifer", "Jeremiah", "Jesse", "Jessica", "Jonah",
                "Jonathan", "Jordan", "Joseph", "Julia",
            ],
        ),
        (
            "K",
            &[
                "Kai", "Kaia", "Kaleb", "Karina", "Karsyn", "Katelyn", "Kayla", "Keanu", "Keith",
                "Kelsey", "Kendall", "Kennedy", "Kenneth", "Kevin", "Kian", "Kimberly", "Kingston",
                "Knox", "Kora", "Kyle",
            ],
        ),
        (
            "L",
            &[
                "Landon", "Lane", "Lauren", "Layla", "Leah", "Leo", "Leonardo", "Levi", "Liam",
                "Lila", "Lincoln", "Linus", "Logan", "Lucas", "Lucy", "Luke", "Luna", "Lydia",
                "Lyla", "Lyric",
            ],
        ),
        (
            "M",
            &[
                "Mackenzie",
                "Madeline",
                "Madison",
                "Maeve",
                "Malachi",
                "Marcus",
                "Margaret",
                "Maria",
                "Mason",
                "Mateo",
                "Matthew",
                "Max",
                "Maya",
                "Melanie",
                "Mia",
                "Michael",
                "Miles",
                "Mila",
                "Milo",
                "Morgan",
            ],
        ),
        (
            "N",
            &[
                "Nadia",
                "Naomi",
                "Nash",
                "Natalia",
                "Natalie",
                "Nathan",
                "Nathaniel",
                "Naya",
                "Neil",
                "Nelson",
                "Neo",
                "Nicholas",
                "Nico",
                "Nicolas",
                "Nicole",
                "Nikolai",
                "Nina",
                "Noah",
                "Noelle",
                "Nolan",
            ],
        ),
        (
            "O",
            &[
                "Oakley", "Odin", "Olive", "Oliver", "Olivia", "Omar", "Ophelia", "Orion",
                "Orlando", "Oscar", "Otis", "Otto", "Owen", "Ozzy", "Oaklyn", "Octavia", "Onyx",
                "Opal", "Oprah", "Orla",
            ],
        ),
        (
            "P",
            &[
                "Paige", "Paisley", "Palmer", "Parker", "Patrice", "Patrick", "Paul", "Paula",
                "Paxton", "Payton", "Penelope", "Percy", "Peter", "Peyton", "Philip", "Phoebe",
                "Phoenix", "Piper", "Poppy", "Preston",
            ],
        ),
        (
            "Q",
            &[
                "Quade", "Queen", "Quentin", "Quest", "Quincy", "Quinn", "Quintin", "Quinton",
                "Quillan", "Quirino", "Quinlan", "Quiana", "Qasim", "Qadir", "Quartus", "Questa",
                "Quinby", "Quinley", "Quill", "Quintana",
            ],
        ),
        (
            "R",
            &[
                "Rachel", "Raiden", "Ralph", "Ramona", "Randall", "Raphael", "Raymond", "Reagan",
                "Reed", "Reese", "Regina", "Reid", "Remi", "Rex", "Rhett", "Rhys", "Riley",
                "River", "Robert", "Rowan",
            ],
        ),
        (
            "S",
            &[
                "Sabrina",
                "Sadie",
                "Sage",
                "Sam",
                "Samantha",
                "Samuel",
                "Santiago",
                "Sara",
                "Sawyer",
                "Scarlett",
                "Sebastian",
                "Selena",
                "Serena",
                "Seth",
                "Silas",
                "Simon",
                "Sloan",
                "Sophia",
                "Stella",
                "Summer",
            ],
        ),
        (
            "T",
            &[
                "Talia", "Tanner", "Tate", "Tatiana", "Taylor", "Teagan", "Teddy", "Theo",
                "Theodore", "Thomas", "Timothy", "Titus", "Tobias", "Travis", "Trent", "Trevor",
                "Tristan", "Tucker", "Tyler", "Tyson",
            ],
        ),
        (
            "U",
            &[
                "Ugo", "Ulani", "Ulla", "Ulric", "Ulysses", "Uma", "Umar", "Umberto", "Unity",
                "Uri", "Uriah", "Uriel", "Ursula", "Usher", "Ustin", "Utah", "Uzair", "Uzziah",
                "Uchenna", "Upendra",
            ],
        ),
        (
            "V",
            &[
                "Valencia",
                "Valentine",
                "Valeria",
                "Valerie",
                "Vance",
                "Vaughn",
                "Veda",
                "Vera",
                "Veronica",
                "Victor",
                "Victoria",
                "Vienna",
                "Vincent",
                "Violet",
                "Virginia",
                "Vivian",
                "Vlad",
                "Vladimir",
                "Von",
                "Vito",
            ],
        ),
        (
            "W",
            &[
                "Wade", "Walker", "Walter", "Warren", "Watson", "Waverly", "Waylon", "Wayne",
                "Wells", "Wesley", "Weston", "Whitney", "Wilder", "Will", "Willa", "William",
                "Willow", "Wilson", "Winston", "Wyatt",
            ],
        ),
        (
            "X",
            &[
                "Xander", "Xanthia", "Xara", "Xavier", "Xena", "Xenia", "Ximena", "Xiomara",
                "Xuan", "Xyle", "Xyla", "Xavi", "Xavion", "Xenon", "Xerxes", "Xiao", "Xia",
                "Xylon", "Xeno", "Xystus",
            ],
        ),
        (
            "Y",
            &[
                "Yadiel", "Yael", "Yahir", "Yale", "Yamila", "Yan", "Yanna", "Yareli", "Yarrow",
                "Yasin", "Yasmin", "Yelena", "Yesenia", "Yohan", "Yosef", "Yuki", "Yusuf", "Yves",
                "Yvette", "Yvonne",
            ],
        ),
        (
            "Z",
            &[
                "Zachariah",
                "Zachary",
                "Zadok",
                "Zahir",
                "Zaid",
                "Zaida",
                "Zain",
                "Zaire",
                "Zak",
                "Zander",
                "Zara",
                "Zaria",
                "Zavier",
                "Zayden",
                "Zayn",
                "Zeke",
                "Zelda",
                "Zion",
                "Zita",
                "Zoe",
            ],
        ),
    ];

    let roles = [
        "Principal Systems Architect",
        "Lead UI Engineer",
        "Rust Graphics Specialist",
        "Product Designer",
        "Compiler Researcher",
        "Platform Infrastructure",
        "Machine Learning Lead",
        "Technical Director",
    ];

    let mut contacts = Vec::with_capacity(520);

    for (letter_str, names) in first_names_by_letter.iter() {
        let letter = letter_str.chars().next().unwrap();
        let color = avatar_color_for_char(letter);

        for (i, &name) in names.iter().enumerate() {
            let role = roles[(letter as usize + i) % roles.len()];
            contacts.push(Contact {
                name: format!("{name} {}", letter_str.repeat(2)),
                role: role.to_string(),
                letter,
                color,
                is_section_start: i == 0,
            });
        }
    }

    contacts
}

fn label(
    content: impl ToString,
    size: f32,
    weight: FontWeight,
    color: Color,
) -> StyledView<Text<AppMsg>> {
    text::<_, AppMsg>(content)
        .style(Style::new().set_text_style(TextStyle::new().size(size).weight(weight).color(color)))
}

fn main() {
    let contacts = generate_sample_contacts();
    let state = AppState {
        contacts,
        is_scrubbing: false,
        scrub_letter: 'A',
        scrub_thumb_y: 100.0,
        scroll_pct: 0.0,
    };

    let mut window = Window::with(state, update, |state: &AppState| {
        // Material 3 slender segmented scrollbar style (Light Theme)
        let m3_scrollbar = ScrollbarStyle {
            width: 6.0,
            margin: 3.0,
            gap: 2.0,
            thumb_color: clr!(0x4f46e5ff),
            track_color: Some(rgba!(0, 0, 0, 18)),
            radius: Radius::all(3.0),
            min_thumb_len: 28.0,
            ..Default::default()
        };

        // Header view (Light Theme Surface)
        let header_view = column((
            row((
                row((label(
                    "MATERIAL 3",
                    10.0,
                    FontWeight::BOLD,
                    clr!(0x4f46e5ff),
                ),))
                .style(
                    Style::new()
                        .padding_xy(8.0, 3.0)
                        .corner_radius(4.0)
                        .bg_color(clr!(0xeef2ffff)),
                ),
                spacer(),
                label(
                    format!(
                        "{}%  {}",
                        (state.scroll_pct * 100.0).round() as i32,
                        if state.is_scrubbing {
                            format!("• Scrubbing [{}]", state.scrub_letter)
                        } else {
                            "• 520 Contacts".to_string()
                        }
                    ),
                    11.0,
                    FontWeight::MEDIUM,
                    if state.is_scrubbing {
                        clr!(0x4f46e5ff)
                    } else {
                        clr!(0x64748bff)
                    },
                ),
            ))
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .align_items(AlignItems::Center),
            ),
            label(
                "Contacts Directory",
                22.0,
                FontWeight::BOLD,
                clr!(0x0f172aff),
            ),
            label(
                "Drag the slender Material 3 scrollbar to scrub smoothly through 520 contacts A–Z.",
                12.0,
                FontWeight::NORMAL,
                clr!(0x64748bff),
            ),
        ))
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .padding_edges(Edges {
                    top: 14.0,
                    bottom: 12.0,
                    left: 20.0,
                    right: 20.0,
                })
                .gap(4.0)
                .bg_color(clr!(0xffffffff))
                .border_bottom(1.0, clr!(0xe2e8f0ff)),
        );

        // Virtualized contact list (56.0 px item height for 120 FPS seamless scrolling)
        let vlist = virtual_list(state.contacts.clone(), 56.0, move |_idx, contact| {
            let section_badge = if contact.is_section_start {
                Some(
                    row((label(
                        format!("SEC {}", contact.letter),
                        10.0,
                        FontWeight::BOLD,
                        clr!(0x4f46e5ff),
                    ),))
                    .style(
                        Style::new()
                            .padding_xy(7.0, 3.0)
                            .corner_radius(4.0)
                            .bg_color(clr!(0xeef2ffff)),
                    ),
                )
            } else {
                None
            };

            row((
                // Circular avatar with initial
                row((label(
                    contact.letter.to_string(),
                    14.0,
                    FontWeight::BOLD,
                    clr!(white),
                ),))
                .style(
                    Style::new()
                        .width(Size::Fixed(36))
                        .height(Size::Fixed(36))
                        .corner_radius(18.0)
                        .bg_color(contact.color)
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::Center),
                ),
                // Contact name and role
                column((
                    label(
                        contact.name.clone(),
                        14.0,
                        FontWeight::SEMI_BOLD,
                        clr!(0x0f172aff),
                    ),
                    label(
                        contact.role.clone(),
                        11.0,
                        FontWeight::NORMAL,
                        clr!(0x64748bff),
                    ),
                ))
                .style(Style::new().gap(2.0)),
                spacer(),
                section_badge,
            ))
            .style(
                Style::new()
                    .width(Size::Percent(1.0))
                    .height(Size::Fixed(56))
                    .min_height(56.0)
                    .max_height(56.0)
                    .padding_xy(16.0, 0.0)
                    .align_items(AlignItems::Center)
                    .gap(12.0)
                    .bg_color(if contact.is_section_start {
                        clr!(0xf8fafcff)
                    } else {
                        clr!(0xffffffff)
                    })
                    .border_bottom(1.0, clr!(0xf1f5f9ff))
                    .on_hover(|s| s.bg_color(clr!(0xf1f5f9ff))),
            )
        })
        .scrollbar(m3_scrollbar)
        .style(
            Style::new()
                .width(Size::Percent(1.0))
                .flex_grow(1.0)
                .flex_shrink(1.0)
                .min_height(0.0)
                .bg_color(clr!(0xffffffff)),
        )
        .on_thumb_scroll(|_state, ctx| Some(AppMsg::ThumbScrolled(ctx)));

        // Floating alphabet scrubber badge (follows thumb during drag scrub)
        let floating_badge = if state.is_scrubbing {
            Some(
                row((label(
                    state.scrub_letter.to_string(),
                    26.0,
                    FontWeight::BOLD,
                    clr!(white),
                ),))
                .style(
                    Style::new()
                        .position(
                            PositionStrategy::absolute()
                                .right(22.0)
                                .top((state.scrub_thumb_y - 24.0).max(95.0)),
                        )
                        .z_index(100)
                        .width(Size::Fixed(52))
                        .height(Size::Fixed(52))
                        .corner_radius(8.0)
                        .bg_color(clr!(0x4f46e5ff))
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::Center)
                        .shadow(Color::new(79, 70, 229, 100), 14.0, 0.4),
                ),
            )
        } else {
            None
        };

        // Main root container (fills window completely, flex column layout)
        column((header_view, vlist, floating_badge)).style(
            Style::new()
                .width(Size::Percent(1.0))
                .height(Size::Percent(1.0))
                .flex_direction(FlexDirection::Column)
                .bg_color(clr!(0xf8f9faff)),
        )
    });

    window.present_with(
        WindowAttributes::default()
            .with_title("MTK Material 3 Alphabet Scrubber (VirtualList)")
            .with_resizable(true)
            .with_size((480, 720).into()),
    );
}
