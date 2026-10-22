import messages from '../../shared/i18n/generated/tour-message-ids.json';
import legacy from './legacy-step-map.json';
import type {HelpMessage} from '../../shared/help/HelpMessage';

/** Titles belong to routes; rich bodies share the same resolver as hover and explicit help. */
export function tourMessage(cardId:string):HelpMessage{
 const id=(messages as Record<string,string>)[cardId];
 if(!id)throw Error(`Missing tour message binding: ${cardId}`);
 return {id} as HelpMessage;
}
export function legacyTourMessage(stepId:string):HelpMessage{
 const id=(legacy as Record<string,string>)[stepId];
 if(!id)throw Error(`Missing legacy tour binding: ${stepId}`);
 return tourMessage(id);
}
