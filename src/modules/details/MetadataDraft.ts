import { useEffect, useState } from 'react';
import type { ImageItem } from '../explorer/types';
import type {DescriptionFormat} from './DescriptionFormat';

type Field = 'description' | 'tags';
interface Draft { key: string; value: string; base: string; format?:DescriptionFormat|null; baseFormat?:DescriptionFormat|null; version: number; identity?: string }
// Keep unsaved edits when switching tabs/photos. Never replace a dirty draft with AI progress.
const drafts = new Map<string, Draft>();

export function useMetadataDraft(item: ImageItem, field: Field) {
  const key = `${item.path}:${field}`;
  const incoming = field === 'description' ? item.description ?? '' : item.tags.join(', ');
  const version = (field === 'description' ? item.descriptionVersion : item.tagsVersion) ?? 0;
  const fresh = (): Draft => ({ key, value: incoming, base: incoming, format:field==='description'?item.descriptionFormat:null,baseFormat:field==='description'?item.descriptionFormat:null,version, identity:item.indexedImageId });
  const dirty=(value:Draft)=>value.value!==value.base||JSON.stringify(value.format??null)!==JSON.stringify(value.baseFormat??null);
  const [draft, setDraft] = useState<Draft>(() => drafts.get(key) ?? fresh());
  useEffect(() => {
    setDraft(current => {
      if (current.key !== key) return drafts.get(key) ?? fresh();
      if (dirty(current)) return current;
      return fresh();
    });
  }, [key, incoming, version, item.indexedImageId,JSON.stringify(item.descriptionFormat)]);
  const effective = draft.key === key ? draft : drafts.get(key) ?? fresh();
  const update = (value: string,format?:DescriptionFormat|null) => {
    const next = { ...effective, value,format:format??null };
    drafts.set(key, next);
    setDraft(next);
  };
  const reset = () => { drafts.delete(key); setDraft(fresh()); };
  const saved = () => {
    const finish = (latest: Draft): Draft => {
      const next = { ...latest, base: effective.value,baseFormat:effective.format, version: effective.version + 1 };
      if(!dirty(next)) drafts.delete(key); else drafts.set(key,next);
      return next;
    };
    // A pending save belongs to the submitted value and photograph. Text typed
    // while awaiting the reply, including after navigating away, stays dirty.
    const retained=drafts.get(key);
    if(retained)finish(retained);
    setDraft(current => current.key===key ? finish(current) : current);
  };
  const sourceChanged=Boolean(effective.identity && item.indexedImageId && effective.identity!==item.indexedImageId);
  return { value: effective.value,format:effective.format, update, reset, saved, version: effective.version, sourceChanged,
    conflict: dirty(effective) && version !== effective.version };
}
