import {text as localeText} from "../../shared/i18n/Runtime";
/** Observations from the same three-mark/four-input fixture, not a universal rating. */
const observations:Record<string,string>={
 get 'fal-ai/flux-2/flash/edit'() { return localeText("editor", "ui.EditorPlacementAccuracy.3454633423428ad2"); },
 get 'bytedance/seedream/v5/lite/edit'() { return localeText("editor", "ui.EditorPlacementAccuracy.5f9e80386c6f509c"); },
 get 'fal-ai/nano-banana-2/edit'() { return localeText("editor", "ui.EditorPlacementAccuracy.17f7cea923454797"); },
 get 'openai/gpt-image-2.5/flare/edit'() { return localeText("editor", "ui.EditorPlacementAccuracy.4f1f346cc184d55d"); }
};
export function placementAccuracy(modelId:string|undefined,operation:string):string{
 if(!['universal-edit','replace-object','relight','recolor'].includes(operation))return localeText("editor", "ui.EditorPlacementAccuracy.224aabbab85c54f7");
 const observed=modelId?observations[modelId]:undefined;
 return observed?observed+localeText("editor", "ui.EditorPlacementAccuracy.6c83d0409fc3092f"):localeText("editor", "ui.EditorPlacementAccuracy.2c8f3733af32c9fc");
}
