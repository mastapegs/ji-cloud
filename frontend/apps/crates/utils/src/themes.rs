use once_cell::sync::OnceCell;
use rgb::RGBA8;
use serde::{
    de::{self, Deserializer},
    Serialize,
    Deserialize,
};
use std::{fmt, marker::PhantomData};

use crate::unwrap::UnwrapJiExt;

pub const THEME_IDS:[ThemeId;3] = [
    ThemeId::None,
    ThemeId::Chalkboard, 
    ThemeId::HappyBrush
];


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ThemeId {
    None,
    Chalkboard,
    HappyBrush,
}


impl ThemeId {

    pub fn get_colors(self) -> &'static [RGBA8] {
        self.map_theme(|theme| theme.colors.as_slice())
    }

    //TODO - tie to Localization
    pub fn display_name(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Chalkboard => "Chalkboard", 
            Self::HappyBrush => "Happy Brush", 
        }
    }

    pub fn as_str_id(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Chalkboard => "chalkboard", 
            Self::HappyBrush => "happy-brush", 
        }
    }

    //It's safe to just call this whenever, it will lazily init the config
    fn map_theme<F, A>(self, mapper:F) -> A 
    where
        F: FnOnce(&'static Theme) -> A
    {
        match THEMES.get() {
            None => {
                init_config();
                self.map_theme(mapper)
            }
            Some(themes) => {
                mapper(match self {
                    Self::None => &themes.none,
                    Self::Chalkboard => &themes.chalkboard,
                    Self::HappyBrush => &themes.happy_brush,
                })
            }
        }
    }

}

//These are for storing the config statically
//access is via the public ThemeId getters
#[derive(Debug, Deserialize)]
struct Themes {
    #[serde(rename="")]
    pub none: Theme,
    pub chalkboard: Theme,
    pub happy_brush: Theme,
}

#[derive(Debug, Deserialize)]
struct Theme {
    #[serde(deserialize_with = "hex_to_rgba8")]
    pub colors: Vec<RGBA8>,
}
//Set lazily, first time as-needed
static THEMES: OnceCell<Themes> = OnceCell::new();

fn init_config() {
    let themes:Themes = serde_json::from_str(include_str!("../../../../config/themes.json")).expect("Invalid Themes");

    THEMES.set(themes).unwrap_ji()
}

//Deserializes the colors from Vec<String> to Vec<RGBA8>
//currently assumes all the strings are in the format 0xRRGGBB
//in the future we can enhance that to support more string types
//without breaking the api
fn hex_to_rgba8<'de, D>(deserializer: D) -> Result<Vec<RGBA8>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ColorVec(PhantomData<Vec<RGBA8>>);

    impl<'de> de::Visitor<'de> for ColorVec {
        type Value = Vec<RGBA8>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("List of Colors as hex values")
        }

        fn visit_seq<S>(self, mut visitor: S) -> Result<Self::Value, S::Error>
        where
            S: de::SeqAccess<'de>,
        {
            let mut out: Vec<RGBA8> = Vec::with_capacity(visitor.size_hint().unwrap_or(0));

            // While there are entries remaining in the input, add them
            // into our vec.
            while let Some(value) = visitor.next_element::<String>()? {
                let value = value.trim_start_matches("0x");
                let value = u32::from_str_radix(value, 16)
                    .map_err(|_| serde::de::Error::custom(format!("invalid color [{}]!", value)))?;

                let r = ((value & 0xFF0000) >> 16) as u8;
                let g = ((value & 0x00FF00) >> 8) as u8;
                let b = (value & 0x0000FF) as u8;
                out.push(RGBA8::new(r, g, b, 255));
            }

            Ok(out)
        }
    }

    deserializer.deserialize_any(ColorVec(PhantomData))
}

