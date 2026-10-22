import {invoke as nativeInvoke,type InvokeArgs,type InvokeOptions} from '@tauri-apps/api/core';
import {i18n} from '../i18n/Runtime';
import captions from '../i18n/native-caption-bindings.json';
import {commandError,commandErrorMessage,technicalError} from './CommandError';
import {nativeDisplayText} from '../i18n/NativeMessages';
const captionCommands=new Set(captions.commands);
export class NativeCommandFailure extends Error{
 readonly code:string;readonly technical:string;readonly retryable?:boolean;readonly original:unknown;readonly statusCode?:number;readonly retryAfterMs?:number;
 constructor(error:unknown){const dto=commandError(error);const technical=technicalError(error);super(nativeDisplayText(commandErrorMessage(error)));this.name='NativeCommandFailure';this.code=dto?.code??'native.operation-failed';this.technical=technical;this.retryable=dto?.retryable;this.original=error;this.statusCode=dto?.statusCode;this.retryAfterMs=dto?.retryAfterMs;}
}
/** Only dialog commands gain an optional UI locale; frozen/provider inputs are untouched. */
export async function invoke<T>(command:string,args?:InvokeArgs,options?:InvokeOptions):Promise<T>{
 const input=captionCommands.has(command)?{...args,uiLocale:i18n.resolvedLanguage??'ru'}:args;
 try{return await nativeInvoke<T>(command,input,options);}catch(error){throw error instanceof Error?error:new NativeCommandFailure(error);}
}
