import availability from '../../shared/config/provider-availability.json';
import {text} from '../../shared/i18n/Runtime';

/** One build-time switch shared with native code. Saved data is not migrated. */
export function isProviderAvailable(provider:string){return provider!=='openrouter'||availability.openrouter;}
export function providerUnavailableMessage(){return text('native','provider.openrouter-unavailable');}
