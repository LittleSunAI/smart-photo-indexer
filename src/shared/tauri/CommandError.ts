import {text} from '../i18n/Runtime';
export interface CommandErrorDto {code:string;technical?:string;message?:string;args?:Record<string,string|number>;retryable?:boolean;statusCode?:number;retryAfterMs?:number}
/** Old native strings remain readable; new boundaries carry stable semantic codes. */
export function commandError(error:unknown):CommandErrorDto|null{
 if(error&&typeof error==='object'&&'code' in error&&typeof error.code==='string')return error as CommandErrorDto;
 if(typeof error==='string'){
  try{const parsed=JSON.parse(error);if(parsed&&typeof parsed.code==='string')return parsed;}catch{}
  // Compatibility only for already-installed native code, never translated UI text.
  if(error==='Выбранное задание больше недоступно.')return {code:'queue.target-missing',technical:error};
  const legacy:Record<string,string>={'Ключ пустой.':'credential.empty','Ключ слишком длинный.':'credential.too-long','Ключ содержит недопустимые символы.':'credential.invalid-characters','Сначала дождитесь сохранения текущего черновика.':'editor.save-pending'};
  if(legacy[error])return {code:legacy[error],technical:error};
 }
 return null;
}
export function commandErrorCode(error:unknown){return commandError(error)?.code;}
export function technicalError(error:unknown):string{
 if(error instanceof Error&&!(typeof (error as unknown as CommandErrorDto).technical==='string'))return error.message;
 const dto=commandError(error);if(dto)return dto.technical??JSON.stringify(dto);
 if(error instanceof Error)return error.message;
 if(typeof error==='string')return error;
 try{return JSON.stringify(error)??String(error);}catch{return String(error);}
}
export function commandErrorMessage(error:unknown):string{
 const dto=commandError(error);
 if(dto?.code==='queue.target-missing')return text('errors','queue.target-missing');
 return dto?.message??technicalError(error);
}
