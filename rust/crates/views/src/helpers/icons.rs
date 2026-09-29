//! IconsAvatarHelper. The icons owner resolves names; views receive its plain, resolved data.
use super::{Attrs, Html, Safe, attrs, content_tag_text, image_tag, value_to_string};
use crate::ViewContext;

#[derive(Clone, Debug)]
pub enum AvatarIcon {
    Emoji {
        title: String,
        character: String,
    },
    Image {
        title: String,
        url: String,
        brand: bool,
    },
}

pub trait IconSource {
    fn resolve_avatar_icon(&self, name: &str) -> Option<AvatarIcon>;
}

pub fn icon_avatar_tag(
    ctx: &ViewContext,
    icon: Option<&AvatarIcon>,
    size: usize,
    mut options: Attrs,
) -> Html {
    let Some(icon) = icon else {
        return Safe(String::new());
    };
    let existing_class = options
        .remove("class")
        .map(|value| value_to_string(&value))
        .unwrap_or_default();
    let kind = match icon {
        AvatarIcon::Emoji { .. } => "emoji",
        AvatarIcon::Image { brand: true, .. } => "brand",
        _ => "custom",
    };
    let class = format!(
        "icon-avatar icon-avatar--{kind}{}",
        if super::is_blank(&existing_class) {
            String::new()
        } else {
            format!(" {existing_class}")
        }
    );
    options = options.class(class);
    match icon {
        AvatarIcon::Emoji { title, character } => {
            let existing_style = options
                .remove("style")
                .map(|value| value_to_string(&value))
                .unwrap_or_default();
            options = options.attr(
                "style",
                format!(
                    "font-size: {size}px{}",
                    if super::is_blank(&existing_style) {
                        String::new()
                    } else {
                        format!("; {existing_style}")
                    }
                ),
            );
            content_tag_text(
                "span",
                attrs()
                    .role("img")
                    .aria("label", title.as_str())
                    .merge(options),
                character,
            )
        }
        AvatarIcon::Image { title, url, .. } => {
            image_tag(ctx, url, attrs().alt(title).size(size).merge(options))
        }
    }
}
