import {nativeDisplayText} from '../../shared/i18n/NativeMessages';
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {EditorModal} from './EditorModal';

export function EditorErrorDialog({message,onClose,title=localeText("editor", "ui.EditorErrorDialog.b58aa71065bc9d75"),returnFocus}:{message:string;onClose():void;title?:string;returnFocus?:HTMLElement|null}){
  useCurrentLocale();
 return <EditorModal title={title} className="editor-error-dialog" onClose={onClose} returnFocus={returnFocus}>
  <p role="alert" className="editor-copyable">{nativeDisplayText(message)}</p>
  <footer><Hint message={{id:"common.dismiss-message"}}><button type="button" onClick={onClose}>{localeText("editor", "ui.EditorErrorDialog.b89bc2ec0af4fcc0")}</button></Hint></footer>
 </EditorModal>;
}
