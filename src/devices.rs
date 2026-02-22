use crate::config::{Insets, PhoneConfig, PhoneModel};

const DEFAULT_CORNER_RADIUS: u32 = 88;
const DEFAULT_INSETS: Insets = Insets {
    top: 28,
    right: 20,
    bottom: 28,
    left: 20,
};
const DEFAULT_FRAME_COLOR: &str = "#11151B";
const DEFAULT_FRAME_BORDER_WIDTH: u32 = 8;
const DEFAULT_SHADOW_OFFSET_Y: i32 = 18;
const DEFAULT_SHADOW_ALPHA: u8 = 74;

pub struct ResolvedPhoneStyle {
    pub corner_radius: u32,
    pub screen_padding: Insets,
    pub frame_color: String,
    pub frame_border_width: u32,
    pub shadow_offset_y: i32,
    pub shadow_alpha: u8,
    pub island: Option<DynamicIslandSpec>,
}

#[derive(Clone, Copy)]
pub struct DynamicIslandSpec {
    pub width_ratio: f32,
    pub height_ratio: f32,
    pub y_offset_ratio: f32,
    pub lens_size_ratio: f32,
}

struct DeviceProfile {
    corner_radius: u32,
    screen_padding: Insets,
    frame_color: &'static str,
    frame_border_width: u32,
    shadow_offset_y: i32,
    shadow_alpha: u8,
    island: Option<DynamicIslandSpec>,
}

pub struct DeviceListing {
    pub slug: &'static str,
    pub display_name: &'static str,
}

pub const DEVICE_LISTINGS: [DeviceListing; 4] = [
    DeviceListing {
        slug: "iphone_16_pro",
        display_name: "iPhone 16 Pro",
    },
    DeviceListing {
        slug: "iphone_16_pro_max",
        display_name: "iPhone 16 Pro Max",
    },
    DeviceListing {
        slug: "iphone_17_pro",
        display_name: "iPhone 17 Pro",
    },
    DeviceListing {
        slug: "iphone_17_pro_max",
        display_name: "iPhone 17 Pro Max",
    },
];

pub fn resolve_phone_style(phone: &PhoneConfig) -> ResolvedPhoneStyle {
    let mut style = ResolvedPhoneStyle {
        corner_radius: phone.corner_radius,
        screen_padding: phone.screen_padding,
        frame_color: phone.frame_color.clone(),
        frame_border_width: phone.frame_border_width,
        shadow_offset_y: phone.shadow_offset_y,
        shadow_alpha: phone.shadow_alpha,
        island: None,
    };

    if let Some(model) = phone.model {
        let profile = profile_for(model);
        style.corner_radius = choose_u32(
            phone.corner_radius,
            DEFAULT_CORNER_RADIUS,
            profile.corner_radius,
        );
        style.screen_padding =
            choose_insets(phone.screen_padding, DEFAULT_INSETS, profile.screen_padding);
        style.frame_color =
            choose_color(&phone.frame_color, DEFAULT_FRAME_COLOR, profile.frame_color);
        style.frame_border_width = choose_u32(
            phone.frame_border_width,
            DEFAULT_FRAME_BORDER_WIDTH,
            profile.frame_border_width,
        );
        style.shadow_offset_y = choose_i32(
            phone.shadow_offset_y,
            DEFAULT_SHADOW_OFFSET_Y,
            profile.shadow_offset_y,
        );
        style.shadow_alpha = choose_u8(
            phone.shadow_alpha,
            DEFAULT_SHADOW_ALPHA,
            profile.shadow_alpha,
        );
        style.island = profile.island;
    }

    style
}

fn profile_for(model: PhoneModel) -> DeviceProfile {
    match model {
        PhoneModel::Iphone16Pro => DeviceProfile {
            corner_radius: 116,
            screen_padding: Insets {
                top: 54,
                right: 28,
                bottom: 40,
                left: 28,
            },
            frame_color: "#7A7F89",
            frame_border_width: 13,
            shadow_offset_y: 24,
            shadow_alpha: 82,
            island: Some(DynamicIslandSpec {
                width_ratio: 0.33,
                height_ratio: 0.050,
                y_offset_ratio: 0.020,
                lens_size_ratio: 0.38,
            }),
        },
        PhoneModel::Iphone16ProMax => DeviceProfile {
            corner_radius: 126,
            screen_padding: Insets {
                top: 54,
                right: 30,
                bottom: 42,
                left: 30,
            },
            frame_color: "#767C86",
            frame_border_width: 14,
            shadow_offset_y: 25,
            shadow_alpha: 83,
            island: Some(DynamicIslandSpec {
                width_ratio: 0.30,
                height_ratio: 0.047,
                y_offset_ratio: 0.020,
                lens_size_ratio: 0.37,
            }),
        },
        PhoneModel::Iphone17Pro => DeviceProfile {
            corner_radius: 122,
            screen_padding: Insets {
                top: 56,
                right: 28,
                bottom: 40,
                left: 28,
            },
            frame_color: "#686F78",
            frame_border_width: 14,
            shadow_offset_y: 25,
            shadow_alpha: 84,
            island: Some(DynamicIslandSpec {
                width_ratio: 0.31,
                height_ratio: 0.046,
                y_offset_ratio: 0.020,
                lens_size_ratio: 0.36,
            }),
        },
        PhoneModel::Iphone17ProMax => DeviceProfile {
            corner_radius: 130,
            screen_padding: Insets {
                top: 56,
                right: 30,
                bottom: 42,
                left: 30,
            },
            frame_color: "#666D76",
            frame_border_width: 15,
            shadow_offset_y: 26,
            shadow_alpha: 85,
            island: Some(DynamicIslandSpec {
                width_ratio: 0.29,
                height_ratio: 0.044,
                y_offset_ratio: 0.020,
                lens_size_ratio: 0.35,
            }),
        },
    }
}

fn choose_u32(input: u32, default_value: u32, device_value: u32) -> u32 {
    if input == default_value {
        device_value
    } else {
        input
    }
}

fn choose_i32(input: i32, default_value: i32, device_value: i32) -> i32 {
    if input == default_value {
        device_value
    } else {
        input
    }
}

fn choose_u8(input: u8, default_value: u8, device_value: u8) -> u8 {
    if input == default_value {
        device_value
    } else {
        input
    }
}

fn choose_insets(input: Insets, default_value: Insets, device_value: Insets) -> Insets {
    Insets {
        top: choose_u32(input.top, default_value.top, device_value.top),
        right: choose_u32(input.right, default_value.right, device_value.right),
        bottom: choose_u32(input.bottom, default_value.bottom, device_value.bottom),
        left: choose_u32(input.left, default_value.left, device_value.left),
    }
}

fn choose_color(input: &str, default_value: &str, device_value: &str) -> String {
    if input.eq_ignore_ascii_case(default_value) {
        device_value.to_string()
    } else {
        input.to_string()
    }
}
