import {text as localeText} from "../../shared/i18n/Runtime";
const images=import.meta.glob('../../assets/help/*.webp',{eager:true,query:'?url',import:'default'}) as Record<string,string>;
export const helpIllustrations: {articleId:string;url:string;caption:string}[]=[
 {articleId:"folders",url:images['../../assets/help/folders.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.92ea37d263087b88"); }},
 {articleId:"refresh",url:images['../../assets/help/refresh.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.2f52f21184ca4602"); }},
 {articleId:"library",url:images['../../assets/help/library.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.22e8d88828211755"); }},
 {articleId:"search",url:images['../../assets/help/search.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.1fba6c396dd4772b"); }},
 {articleId:"recognition",url:images['../../assets/help/recognition.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.c94cc29719a55594"); }},
 {articleId:"privacy",url:images['../../assets/help/privacy.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.d7b2d524fa321fd3"); }},
 {articleId:"queue",url:images['../../assets/help/queue.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.e1dc64c562112723"); }},
 {articleId:"inspector",url:images['../../assets/help/inspector.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.84864551419aa32c"); }},
 {articleId:"viewer",url:images['../../assets/help/viewer.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.2853a62d03f17cd4"); }},
 {articleId:"editor",url:images['../../assets/help/editor.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.94219d249aa9c23c"); }},
 {articleId:"results",url:images['../../assets/help/results.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.e7b06547f997e8f5"); }},
 {articleId:"appearance",url:images['../../assets/help/help.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.20d034d29d7715a3"); }},
 {articleId:"rename",url:images['../../assets/help/rename.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.dc78e6f7167218dc"); }},
 {articleId:"editor-tasks",url:images['../../assets/help/editor-tasks.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.637b235ff8fa635d"); }},
 {articleId:"editor-instruction-guide",url:images['../../assets/help/editor.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.ac2dad281765206f"); }},
 {articleId:"editor-safe-workflow",url:images['../../assets/help/editor.webp'],get caption() { return localeText("main", "ui.HelpIllustrations.fb8a154feed787ea"); }},
];
