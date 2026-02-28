use once_cell::sync::Lazy;
use std::collections::HashMap;

static WECHAT_TO_UNICODE: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("[微笑]", "🙂");
    map.insert("[撇嘴]", "🙎");
    map.insert("[色]", "😍");
    map.insert("[发呆]", "😐");
    map.insert("[得意]", "😏");
    map.insert("[流泪]", "😢");
    map.insert("[害羞]", "😊");
    map.insert("[闭嘴]", "😶");
    map.insert("[睡]", "😴");
    map.insert("[大哭]", "😭");
    map.insert("[尴尬]", "😅");
    map.insert("[发怒]", "😠");
    map.insert("[调皮]", "😜");
    map.insert("[呲牙]", "😁");
    map.insert("[惊讶]", "😲");
    map.insert("[难过]", "😔");
    map.insert("[酷]", "😎");
    map.insert("[冷汗]", "😰");
    map.insert("[抓狂]", "😫");
    map.insert("[吐]", "🤮");
    map.insert("[偷笑]", "🤭");
    map.insert("[愉快]", "😊");
    map.insert("[白眼]", "🙄");
    map.insert("[傲慢]", "😤");
    map.insert("[饥饿]", "🤤");
    map.insert("[困]", "😪");
    map.insert("[惊恐]", "😱");
    map.insert("[流汗]", "😓");
    map.insert("[憨笑]", "😃");
    map.insert("[悠闲]", "😌");
    map.insert("[奋斗]", "💪");
    map.insert("[咒骂]", "🤬");
    map.insert("[疑问]", "❓");
    map.insert("[嘘]", "🤫");
    map.insert("[晕]", "😵");
    map.insert("[疯了]", "🤪");
    map.insert("[衰]", "☹️");
    map.insert("[骷髅]", "💀");
    map.insert("[敲打]", "敲打");
    map.insert("[再见]", "👋");
    map.insert("[擦汗]", "😅");
    map.insert("[抠鼻]", "🤔");
    map.insert("[鼓掌]", "👏");
    map.insert("[糗大了]", "😳");
    map.insert("[坏笑]", "🤭");
    map.insert("[左哼哼]", "😤");
    map.insert("[右哼哼]", "😤");
    map.insert("[哈欠]", "🥱");
    map.insert("[鄙视]", "😒");
    map.insert("[委屈]", "🥺");
    map.insert("[快哭了]", "😢");
    map.insert("[阴险]", "😏");
    map.insert("[亲亲]", "😘");
    map.insert("[吓]", "😨");
    map.insert("[可怜]", "🥺");
    map.insert("[菜刀]", "🔪");
    map.insert("[西瓜]", "🍉");
    map.insert("[啤酒]", "🍺");
    map.insert("[篮球]", "🏀");
    map.insert("[乒乓]", "🏓");
    map.insert("[咖啡]", "☕");
    map.insert("[饭]", "🍚");
    map.insert("[猪头]", "🐷");
    map.insert("[玫瑰]", "🌹");
    map.insert("[凋谢]", "🥀");
    map.insert("[嘴唇]", "👄");
    map.insert("[爱心]", "❤️");
    map.insert("[心碎]", "💔");
    map.insert("[蛋糕]", "🎂");
    map.insert("[闪电]", "⚡");
    map.insert("[炸弹]", "💣");
    map.insert("[刀]", "🔪");
    map.insert("[足球]", "⚽");
    map.insert("[瓢虫]", "🐞");
    map.insert("[便便]", "💩");
    map.insert("[月亮]", "🌙");
    map.insert("[太阳]", "☀️");
    map.insert("[礼物]", "🎁");
    map.insert("[拥抱]", "🤗");
    map.insert("[强]", "👍");
    map.insert("[弱]", "👎");
    map.insert("[握手]", "🤝");
    map.insert("[胜利]", "✌️");
    map.insert("[抱拳]", "🙏");
    map.insert("[勾引]", "👉");
    map.insert("[拳头]", "👊");
    map.insert("[差劲]", "👎");
    map.insert("[爱你]", "🤟");
    map.insert("[NO]", "🙅");
    map.insert("[OK]", "👌");
    map.insert("[爱情]", "💑");
    map.insert("[飞吻]", "😘");
    map.insert("[跳跳]", "蹦跳");
    map.insert("[发抖]", "🫨");
    map.insert("[怄火]", "火");
    map.insert("[转圈]", "旋转");
    map.insert("[磕头]", "🙇");
    map.insert("[回头]", "👀");
    map.insert("[跳绳]", "🏃");
    map.insert("[激动]", "🤩");
    map.insert("[街舞]", "街舞");
    map.insert("[献吻]", "💋");
    map.insert("[左太极]", "太极");
    map.insert("[右太极]", "太极");
    map.insert("[双喜]", "囍");
    map.insert("[鞭炮]", "🧨");
    map.insert("[灯笼]", "🏮");
    map.insert("[发财]", "🧧");
    map.insert("[K歌]", "🎤");
    map.insert("[购物]", "🛒");
    map.insert("[邮件]", "📧");
    map.insert("[帅气]", "帅");
    map.insert("[喝彩]", "🎉");
    map.insert("[祈祷]", "🙏");
    map.insert("[爆筋]", "💪");
    map.insert("[棒棒糖]", "🍭");
    map.insert("[喝奶]", "🍼");
    map.insert("[面条]", "🍜");
    map.insert("[香蕉]", "🍌");
    map.insert("[飞机]", "✈️");
    map.insert("[汽车]", "🚗");
    map.insert("[火车]", "🚂");
    map.insert("[公交]", "🚌");
    map.insert("[轮船]", "🚢");
    map.insert("[钞票]", "💵");
    map.insert("[熊猫]", "🐼");
    map.insert("[兔子]", "🐰");
    map.insert("[_Onerous]", "😫");
    map
});

static UNICODE_TO_WECHAT: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for (k, v) in WECHAT_TO_UNICODE.iter() {
        map.insert(*v, *k);
    }
    map
});

pub fn wechat_to_unicode(text: &str) -> String {
    let mut result = text.to_string();
    for (wechat, unicode) in WECHAT_TO_UNICODE.iter() {
        result = result.replace(wechat, unicode);
    }
    result
}

pub fn unicode_to_wechat(text: &str) -> String {
    let mut result = text.to_string();
    for (unicode, wechat) in UNICODE_TO_WECHAT.iter() {
        result = result.replace(unicode, wechat);
    }
    result
}
