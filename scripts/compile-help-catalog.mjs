// Compatibility entry point: author text lives in src/shared/i18n/locales.
import {compileLocales} from './compile-locales.mjs';
console.log(JSON.stringify(await compileLocales({check:process.argv.includes('--check')})));
