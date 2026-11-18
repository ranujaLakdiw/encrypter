import * as wasm from "./huh_encrypter_bg.wasm.js";
import { __wbg_set_wasm } from "./huh_encrypter_bg.js";

__wbg_set_wasm(wasm);
wasm.__wbindgen_start();
export {
    IntoUnderlyingByteSource, IntoUnderlyingSink, IntoUnderlyingSource, start
} from "./huh_encrypter_bg.js";
