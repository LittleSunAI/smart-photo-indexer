import {invoke} from "../../shared/tauri/invoke";
import {isProviderAvailable,providerUnavailableMessage} from './ProviderAvailability';

export interface ProviderStatus {ready?:boolean;message?:string}
const requests=new Map<string,Promise<ProviderStatus>>();
export function forgetProviderReadiness(){requests.clear();}
/** Startup and all buttons share this promise; native code owns credential/model validation. */
export function prepareSavedProvider(provider:string,model:string,credentialRef:string){
 if(!isProviderAvailable(provider))return Promise.resolve({ready:false,message:providerUnavailableMessage()});
 const key=`${provider}|${model}|${credentialRef}`;
 let request=requests.get(key);
 if(!request){request=invoke<ProviderStatus>('ensure_ai_provider_ready',{provider,model,credentialRef});requests.set(key,request);}
 return request;
}
