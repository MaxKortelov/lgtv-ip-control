#[derive(Debug)]
pub enum ConnectionType {
    Wired,
    Wifi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Apps {
    Amazon,
    GooglePlay,
    Hulu,
    Netflix,
    SlingTV,
    Youtube,
    Vudu,
    Settings,
    Photos,
    Music,
    Guide,
    Browser,
    Gallery,
    Plex,
    Disney,
    HboMax,
}

impl Apps {
    pub fn as_str(&self) -> &'static str {
        match self {
            Apps::Amazon => "amazon",
            Apps::GooglePlay => "googleplaymovieswebos",
            Apps::Hulu => "hulu",
            Apps::Netflix => "netflix",
            Apps::SlingTV => "com.movenetworks.app.sling-tv-sling-production",
            Apps::Youtube => "youtube.leanback.v4",
            Apps::Vudu => "vudu",
            Apps::Settings => "com.palm.app.settings",
            Apps::Photos => "com.webos.app.photovideo",
            Apps::Music => "com.webos.app.music",
            Apps::Guide => "com.webos.service.iepg",
            Apps::Browser => "com.webos.app.browser",
            Apps::Gallery => "com.webos.app.igallery",
            Apps::Plex => "cdp-30",
            Apps::Disney => "com.disney.disneyplus-prod",
            Apps::HboMax => "com.hbo.hbomax",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergySavingLevels {
    Auto,
    ScreenOff,
    Maximum,
    Medium,
    Minimum,
    Off,
}

impl EnergySavingLevels {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnergySavingLevels::Auto => "auto",
            EnergySavingLevels::ScreenOff => "screenoff",
            EnergySavingLevels::Maximum => "maximum",
            EnergySavingLevels::Medium => "medium",
            EnergySavingLevels::Minimum => "minimum",
            EnergySavingLevels::Off => "off",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inputs {
    Dtv,
    Atv,
    Cadtv,
    Catv,
    Av,
    Component,
    Hdmi1,
    Hdmi2,
    Hdmi3,
    Hdmi4,
}

impl Inputs {
    pub fn as_str(&self) -> &'static str {
        match self {
            Inputs::Dtv => "dtv",
            Inputs::Atv => "atv",
            Inputs::Cadtv => "cadtv",
            Inputs::Catv => "catv",
            Inputs::Av => "avav1",
            Inputs::Component => "component1",
            Inputs::Hdmi1 => "hdmi1",
            Inputs::Hdmi2 => "hdmi2",
            Inputs::Hdmi3 => "hdmi3",
            Inputs::Hdmi4 => "hdmi4",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keys {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    AspectRatio,
    AudioMode,
    Back,
    BlueButton,
    CaptionSubtitle,
    ChannelDown,
    ChannelList,
    ChannelUp,
    DeviceInput,
    EnergySaving,
    FastForward,
    GreenButton,
    Home,
    Info,
    LiveTV,
    Menu,
    Number0,
    Number1,
    Number2,
    Number3,
    Number4,
    Number5,
    Number6,
    Number7,
    Number8,
    Number9,
    Ok,
    Play,
    PreviousChannel,
    ProgramGuide,
    Record,
    RedButton,
    Rewind,
    SleepTimer,
    UserGuide,
    VideoMode,
    VolumeDown,
    VolumeMute,
    VolumeUp,
    YellowButton,
}

impl Keys {
    pub fn as_str(&self) -> &'static str {
        match self {
            Keys::ArrowDown => "arrowdown",
            Keys::ArrowLeft => "arrowleft",
            Keys::ArrowRight => "arrowright",
            Keys::ArrowUp => "arrowup",
            Keys::AspectRatio => "aspectratio",
            Keys::AudioMode => "audiomode",
            Keys::Back => "returnback",
            Keys::BlueButton => "bluebutton",
            Keys::CaptionSubtitle => "captionsubtitle",
            Keys::ChannelDown => "channeldown",
            Keys::ChannelList => "channellist",
            Keys::ChannelUp => "channelup",
            Keys::DeviceInput => "deviceinput",
            Keys::EnergySaving => "screenbright",
            Keys::FastForward => "fastforward",
            Keys::GreenButton => "greenbutton",
            Keys::Home => "myapp",
            Keys::Info => "programminfo",
            Keys::LiveTV => "livetv",
            Keys::Menu => "settingmenu",
            Keys::Number0 => "number0",
            Keys::Number1 => "number1",
            Keys::Number2 => "number2",
            Keys::Number3 => "number3",
            Keys::Number4 => "number4",
            Keys::Number5 => "number5",
            Keys::Number6 => "number6",
            Keys::Number7 => "number7",
            Keys::Number8 => "number8",
            Keys::Number9 => "number9",
            Keys::Ok => "ok",
            Keys::Play => "play",
            Keys::PreviousChannel => "previouschannel",
            Keys::ProgramGuide => "programguide",
            Keys::Record => "record",
            Keys::RedButton => "redbutton",
            Keys::Rewind => "rewind",
            Keys::SleepTimer => "sleepreserve",
            Keys::UserGuide => "userguide",
            Keys::VideoMode => "videomode",
            Keys::VolumeDown => "volumedown",
            Keys::VolumeMute => "volumemute",
            Keys::VolumeUp => "volumeup",
            Keys::YellowButton => "yellowbutton",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PictureModes {
    Cinema,
    Eco,
    FilmMaker,
    Game,
    Normal,
    Sports,
    Vivid,
}

impl PictureModes {
    pub fn as_str(&self) -> &'static str {
        match self {
            PictureModes::Cinema => "cinema",
            PictureModes::Eco => "eco",
            PictureModes::FilmMaker => "filmMaker",
            PictureModes::Game => "game",
            PictureModes::Normal => "normal",
            PictureModes::Sports => "sports",
            PictureModes::Vivid => "vivid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerStates {
    On,
    Off,
    Unknown,
}

impl PowerStates {
    pub fn as_str(&self) -> &'static str {
        match self {
            PowerStates::On => "on",
            PowerStates::Off => "off",
            PowerStates::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenMuteModes {
    ScreenMuteOn,
    VideoMuteOn,
    AllMuteOff,
}

impl ScreenMuteModes {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScreenMuteModes::ScreenMuteOn => "screenmuteon",
            ScreenMuteModes::VideoMuteOn => "videomuteon",
            ScreenMuteModes::AllMuteOff => "allmuteoff",
        }
    }
}

#[derive(Debug)]
pub struct AppDetails {
    pub app: String,
    pub hot_plug: String,
    pub signal: String,
    pub hdcp_version: String,
    pub hdcp_status: String,
}

//Volume level type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VolumeLevel(u8);

impl VolumeLevel {
    pub const MIN: u8 = 0;
    pub const MAX: u8 = 100;

    pub fn new(level: u8) -> Result<Self, &'static str> {
        if level <= Self::MAX {
            Ok(Self(level))
        } else {
            Err("volume level must be between 0 and 100")
        }
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for VolumeLevel {
    type Error = &'static str;

    fn try_from(level: u8) -> Result<Self, Self::Error> {
        Self::new(level)
    }
}

impl From<VolumeLevel> for u8 {
    fn from(level: VolumeLevel) -> Self {
        level.value()
    }
}
