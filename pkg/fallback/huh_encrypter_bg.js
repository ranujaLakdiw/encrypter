export class IntoUnderlyingByteSource {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingByteSourceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingbytesource_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get autoAllocateChunkSize() {
        const ret = wasm.intounderlyingbytesource_autoAllocateChunkSize(this.__wbg_ptr);
        return ret >>> 0;
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
    /**
     * @param {ReadableByteStreamController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
    /**
     * @param {ReadableByteStreamController} controller
     */
    start(controller) {
        wasm.intounderlyingbytesource_start(this.__wbg_ptr, addHeapObject(controller));
    }
    /**
     * @returns {ReadableStreamType}
     */
    get type() {
        const ret = wasm.intounderlyingbytesource_type(this.__wbg_ptr);
        return __wbindgen_enum_ReadableStreamType[ret];
    }
}
if (Symbol.dispose) IntoUnderlyingByteSource.prototype[Symbol.dispose] = IntoUnderlyingByteSource.prototype.free;

export class IntoUnderlyingSink {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSinkFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsink_free(ptr, 0);
    }
    /**
     * @param {any} reason
     * @returns {Promise<any>}
     */
    abort(reason) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_abort(ptr, addHeapObject(reason));
        return takeObject(ret);
    }
    /**
     * @returns {Promise<any>}
     */
    close() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_close(ptr);
        return takeObject(ret);
    }
    /**
     * @param {any} chunk
     * @returns {Promise<any>}
     */
    write(chunk) {
        const ret = wasm.intounderlyingsink_write(this.__wbg_ptr, addHeapObject(chunk));
        return takeObject(ret);
    }
}
if (Symbol.dispose) IntoUnderlyingSink.prototype[Symbol.dispose] = IntoUnderlyingSink.prototype.free;

export class IntoUnderlyingSource {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSourceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsource_free(ptr, 0);
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingsource_cancel(ptr);
    }
    /**
     * @param {ReadableStreamDefaultController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, addHeapObject(controller));
        return takeObject(ret);
    }
}
if (Symbol.dispose) IntoUnderlyingSource.prototype[Symbol.dispose] = IntoUnderlyingSource.prototype.free;

export function start() {
    wasm.start();
}
export function __wbg___wbindgen_debug_string_f249a2a76caa9a55(arg0, arg1) {
    const ret = debugString(getObject(arg1));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
}
export function __wbg___wbindgen_is_falsy_d864b623500ae65d(arg0) {
    const ret = !getObject(arg0);
    return ret;
}
export function __wbg___wbindgen_is_function_741ff2bb612836f5(arg0) {
    const ret = typeof(getObject(arg0)) === 'function';
    return ret;
}
export function __wbg___wbindgen_is_null_c22311b1cc228e3a(arg0) {
    const ret = getObject(arg0) === null;
    return ret;
}
export function __wbg___wbindgen_is_string_fc456fee5d3317de(arg0) {
    const ret = typeof(getObject(arg0)) === 'string';
    return ret;
}
export function __wbg___wbindgen_is_undefined_be5b34249ff1fc15(arg0) {
    const ret = getObject(arg0) === undefined;
    return ret;
}
export function __wbg___wbindgen_string_get_7ce8eee7fd327545(arg0, arg1) {
    const obj = getObject(arg1);
    const ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
}
export function __wbg___wbindgen_throw_4c88d8bf264fd984(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
}
export function __wbg__wbg_cb_unref_09736879a3f67559(arg0) {
    getObject(arg0)._wbg_cb_unref();
}
export function __wbg_activeElement_6c30fe7fda33ef51(arg0) {
    const ret = getObject(arg0).activeElement;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_addEventListener_847fd86b8c33ec43() { return handleError(function (arg0, arg1, arg2, arg3) {
    getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
}, arguments); }
export function __wbg_add_cf82c8d153d93ae1() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).add(getStringFromWasm0(arg1, arg2));
}, arguments); }
export function __wbg_appendChild_dbf7ad2f48413560() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).appendChild(getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_arrayBuffer_5abbcb61bd48f427(arg0) {
    const ret = getObject(arg0).arrayBuffer();
    return addHeapObject(ret);
}
export function __wbg_body_5f971cbc6950ca4f(arg0) {
    const ret = getObject(arg0).body;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_buffer_aca05a93c046db1a(arg0) {
    const ret = getObject(arg0).buffer;
    return addHeapObject(ret);
}
export function __wbg_byobRequest_18280c78e462544c(arg0) {
    const ret = getObject(arg0).byobRequest;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_byteLength_c5f3c188d5252e5b(arg0) {
    const ret = getObject(arg0).byteLength;
    return ret;
}
export function __wbg_byteOffset_63cdc7b38f33a45e(arg0) {
    const ret = getObject(arg0).byteOffset;
    return ret;
}
export function __wbg_call_40b008ba32f73991() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).call(getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_call_a410eb4b59ed5965() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_call_fc92fee6137d452d() { return handleError(function (arg0, arg1, arg2, arg3) {
    const ret = getObject(arg0).call(getObject(arg1), getObject(arg2), getObject(arg3));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_cancelBubble_1164dd588f0c3481(arg0) {
    const ret = getObject(arg0).cancelBubble;
    return ret;
}
export function __wbg_charCodeAt_851946d8f1279e11(arg0, arg1) {
    const ret = getObject(arg0).charCodeAt(arg1 >>> 0);
    return ret;
}
export function __wbg_classList_97e966c261669b85(arg0) {
    const ret = getObject(arg0).classList;
    return addHeapObject(ret);
}
export function __wbg_clearTimeout_fd2c5dfcf4c67f84(arg0, arg1) {
    getObject(arg0).clearTimeout(arg1);
}
export function __wbg_click_c6eff13daacf9806(arg0) {
    getObject(arg0).click();
}
export function __wbg_clientHeight_b40e8971aed9fa2b(arg0) {
    const ret = getObject(arg0).clientHeight;
    return ret;
}
export function __wbg_clientWidth_269ea87f17b863bf(arg0) {
    const ret = getObject(arg0).clientWidth;
    return ret;
}
export function __wbg_clipboardData_40289949920202d6(arg0) {
    const ret = getObject(arg0).clipboardData;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_cloneNode_36b90700fd8dbbb0() { return handleError(function (arg0) {
    const ret = getObject(arg0).cloneNode();
    return addHeapObject(ret);
}, arguments); }
export function __wbg_cloneNode_70249c8beed88ee3() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).cloneNode(arg1 !== 0);
    return addHeapObject(ret);
}, arguments); }
export function __wbg_close_294d3f727cb7a844() { return handleError(function (arg0) {
    getObject(arg0).close();
}, arguments); }
export function __wbg_close_4c46ff8802ca60e6(arg0) {
    getObject(arg0).close();
}
export function __wbg_close_bf94aa7e38b9a94b() { return handleError(function (arg0) {
    getObject(arg0).close();
}, arguments); }
export function __wbg_closest_39824b16065066bc() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).closest(getStringFromWasm0(arg1, arg2));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_composedPath_99300fc960717ac7(arg0) {
    const ret = getObject(arg0).composedPath();
    return addHeapObject(ret);
}
export function __wbg_content_6fed93ca1fd29f06(arg0) {
    const ret = getObject(arg0).content;
    return addHeapObject(ret);
}
export function __wbg_createComment_bf8f324d5ce3d460(arg0, arg1, arg2) {
    const ret = getObject(arg0).createComment(getStringFromWasm0(arg1, arg2));
    return addHeapObject(ret);
}
export function __wbg_createElementNS_2f7464180077e8e4() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    const ret = getObject(arg0).createElementNS(arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_createElement_de4a31ed2fb252be() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).createElement(getStringFromWasm0(arg1, arg2));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_createObjectURL_45693a1454d24dac() { return handleError(function (arg0, arg1) {
    const ret = URL.createObjectURL(getObject(arg1));
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
}, arguments); }
export function __wbg_createTextNode_2b48c3fc75dfcc67(arg0, arg1, arg2) {
    const ret = getObject(arg0).createTextNode(getStringFromWasm0(arg1, arg2));
    return addHeapObject(ret);
}
export function __wbg_crypto_56d59c4d711676e1() { return handleError(function (arg0) {
    const ret = getObject(arg0).crypto;
    return addHeapObject(ret);
}, arguments); }
export function __wbg_dataTransfer_3f478a3e477952f6(arg0) {
    const ret = getObject(arg0).dataTransfer;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_deleteProperty_3ad7a085da623822() { return handleError(function (arg0, arg1) {
    const ret = Reflect.deleteProperty(getObject(arg0), getObject(arg1));
    return ret;
}, arguments); }
export function __wbg_document_3b5c3071b43e97ee(arg0) {
    const ret = getObject(arg0).document;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_enqueue_c11626eae1c54509() { return handleError(function (arg0, arg1) {
    getObject(arg0).enqueue(getObject(arg1));
}, arguments); }
export function __wbg_error_757e9472f8410341(arg0, arg1) {
    let deferred0_0;
    let deferred0_1;
    try {
        deferred0_0 = arg0;
        deferred0_1 = arg1;
        console.error(getStringFromWasm0(arg0, arg1));
    } finally {
        wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
    }
}
export function __wbg_execCommand_d0b4cb02ada3b997() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).execCommand(getStringFromWasm0(arg1, arg2));
    return ret;
}, arguments); }
export function __wbg_files_2d8b9138ca028e2c(arg0) {
    const ret = getObject(arg0).files;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_files_d7e65f35a3acc89f(arg0) {
    const ret = getObject(arg0).files;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_firstElementChild_a60f6eaf7d453487(arg0) {
    const ret = getObject(arg0).firstElementChild;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_focus_0a8d513ea5d8338e() { return handleError(function (arg0, arg1) {
    getObject(arg0).focus(getObject(arg1));
}, arguments); }
export function __wbg_focus_d5e2fdbdb56416f0() { return handleError(function (arg0) {
    getObject(arg0).focus();
}, arguments); }
export function __wbg_getRandomValues_07f3179f2d015362() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).getRandomValues(getArrayU8FromWasm0(arg1, arg2));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_get_2080c49015e35254() { return handleError(function (arg0, arg1) {
    const ret = Reflect.get(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_get_955bde2b2fe2defa(arg0, arg1) {
    const ret = getObject(arg0)[arg1 >>> 0];
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_get_d7176cfb76d71114(arg0, arg1) {
    const ret = getObject(arg0)[arg1 >>> 0];
    return addHeapObject(ret);
}
export function __wbg_hasAttribute_3f271bd99815d507(arg0, arg1, arg2) {
    const ret = getObject(arg0).hasAttribute(getStringFromWasm0(arg1, arg2));
    return ret;
}
export function __wbg_host_64d73c152c626ec8(arg0) {
    const ret = getObject(arg0).host;
    return addHeapObject(ret);
}
export function __wbg_includes_40dae55ee37516e5(arg0, arg1, arg2) {
    const ret = getObject(arg0).includes(getObject(arg1), arg2);
    return ret;
}
export function __wbg_insertBefore_10ab0c19b463c12b() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).insertBefore(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_instanceof_DragEvent_6f36525d948c6b32(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof DragEvent;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_Element_05dbb22ded594343(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Element;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_Error_74e3cca0c4a69161(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Error;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_HtmlDocument_9a3864a99ea4f197(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof HTMLDocument;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_HtmlElement_ea32ea7121e0a496(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof HTMLElement;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_HtmlTextAreaElement_37c6efee878106d5(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof HTMLTextAreaElement;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_Promise_59148c6049d0e3a0(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Promise;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_ShadowRoot_cd78159e03fed884(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof ShadowRoot;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_instanceof_Window_5cf453f5639aa1a2(arg0) {
    let result;
    try {
        result = getObject(arg0) instanceof Window;
    } catch (_) {
        result = false;
    }
    const ret = result;
    return ret;
}
export function __wbg_isSecureContext_6b3c96e6b1051931(arg0) {
    const ret = getObject(arg0).isSecureContext;
    return ret;
}
export function __wbg_is_2248ff2b575d60c7(arg0, arg1) {
    const ret = Object.is(getObject(arg0), getObject(arg1));
    return ret;
}
export function __wbg_length_186163212ffe79c3(arg0) {
    const ret = getObject(arg0).length;
    return ret;
}
export function __wbg_length_e83e819f48c774e4(arg0) {
    const ret = getObject(arg0).length;
    return ret;
}
export function __wbg_matchMedia_b53687d22ec43649() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).matchMedia(getStringFromWasm0(arg1, arg2));
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}, arguments); }
export function __wbg_matches_a47c420646498c64(arg0) {
    const ret = getObject(arg0).matches;
    return ret;
}
export function __wbg_message_43d8c8f1d543a349(arg0) {
    const ret = getObject(arg0).message;
    return addHeapObject(ret);
}
export function __wbg_name_f721590be23206c9(arg0, arg1) {
    const ret = getObject(arg1).name;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
}
export function __wbg_navigator_c17f443a6fe5d38d(arg0) {
    const ret = getObject(arg0).navigator;
    return addHeapObject(ret);
}
export function __wbg_new_13de428334042d26(arg0) {
    const ret = new Uint8Array(getObject(arg0));
    return addHeapObject(ret);
}
export function __wbg_new_787b9fb330d55d78(arg0, arg1) {
    const ret = new Error(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
}
export function __wbg_new_ba40181b3f24ac80(arg0, arg1) {
    try {
        var state0 = {a: arg0, b: arg1};
        var cb0 = (arg0, arg1) => {
            const a = state0.a;
            state0.a = 0;
            try {
                return wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___js_sys_e915b9bba08aaee2___Function_fn_wasm_bindgen_177ff7d6ee1925ce___JsValue_____wasm_bindgen_177ff7d6ee1925ce___sys__Undefined___js_sys_e915b9bba08aaee2___Function_fn_wasm_bindgen_177ff7d6ee1925ce___JsValue_____wasm_bindgen_177ff7d6ee1925ce___sys__Undefined_______true_(a, state0.b, arg0, arg1);
            } finally {
                state0.a = a;
            }
        };
        const ret = new Promise(cb0);
        return addHeapObject(ret);
    } finally {
        state0.a = 0;
    }
}
export function __wbg_new_bb74001155cb8ca8() {
    const ret = new Error();
    return addHeapObject(ret);
}
export function __wbg_new_cd4348290afae586() {
    const ret = new Object();
    return addHeapObject(ret);
}
export function __wbg_new_from_slice_b8daf545a41c6aa0(arg0, arg1) {
    const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
    return addHeapObject(ret);
}
export function __wbg_new_typed_e09da9dae6ee5935(arg0, arg1) {
    try {
        var state0 = {a: arg0, b: arg1};
        var cb0 = (arg0, arg1) => {
            const a = state0.a;
            state0.a = 0;
            try {
                return wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___js_sys_e915b9bba08aaee2___Function_fn_wasm_bindgen_177ff7d6ee1925ce___JsValue_____wasm_bindgen_177ff7d6ee1925ce___sys__Undefined___js_sys_e915b9bba08aaee2___Function_fn_wasm_bindgen_177ff7d6ee1925ce___JsValue_____wasm_bindgen_177ff7d6ee1925ce___sys__Undefined_______true_(a, state0.b, arg0, arg1);
            } finally {
                state0.a = a;
            }
        };
        const ret = new Promise(cb0);
        return addHeapObject(ret);
    } finally {
        state0.a = 0;
    }
}
export function __wbg_new_with_byte_offset_and_length_3cd32055a417f095(arg0, arg1, arg2) {
    const ret = new Uint8Array(getObject(arg0), arg1 >>> 0, arg2 >>> 0);
    return addHeapObject(ret);
}
export function __wbg_new_with_str_sequence_and_options_ea4ce88081790553() { return handleError(function (arg0, arg1) {
    const ret = new Blob(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_new_with_u8_array_sequence_and_options_ca80bf9b8bbc67cf() { return handleError(function (arg0, arg1) {
    const ret = new Blob(getObject(arg0), getObject(arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_now_463ac581d2fc7ea3() {
    const ret = Date.now();
    return ret;
}
export function __wbg_of_027eb15371aac932(arg0) {
    const ret = Array.of(getObject(arg0));
    return addHeapObject(ret);
}
export function __wbg_offsetWidth_ec7d4ad6adc7e29d(arg0) {
    const ret = getObject(arg0).offsetWidth;
    return ret;
}
export function __wbg_open_231cf34b636a3195(arg0) {
    const ret = getObject(arg0).open;
    return ret;
}
export function __wbg_parentNode_fe15bcc26df42856(arg0) {
    const ret = getObject(arg0).parentNode;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_parse_c3f72b5a24f83c01() { return handleError(function (arg0, arg1) {
    const ret = JSON.parse(getStringFromWasm0(arg0, arg1));
    return addHeapObject(ret);
}, arguments); }
export function __wbg_preventDefault_cfb06b02e7406899(arg0) {
    getObject(arg0).preventDefault();
}
export function __wbg_prototypesetcall_13162965d1fe7aa3(arg0, arg1, arg2) {
    Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), getObject(arg2));
}
export function __wbg_queueMicrotask_1e8503a8a690f0a3(arg0) {
    const ret = getObject(arg0).queueMicrotask;
    return addHeapObject(ret);
}
export function __wbg_queueMicrotask_4a1a4d8dd332373b(arg0) {
    queueMicrotask(getObject(arg0));
}
export function __wbg_removeAttribute_735f25a801a701e1() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).removeAttribute(getStringFromWasm0(arg1, arg2));
}, arguments); }
export function __wbg_removeEventListener_4e20bd5fac3de092() { return handleError(function (arg0, arg1, arg2, arg3) {
    getObject(arg0).removeEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
}, arguments); }
export function __wbg_remove_12e9b1deffec7d2f() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).remove(getStringFromWasm0(arg1, arg2));
}, arguments); }
export function __wbg_remove_8228e91d7ddbc755(arg0) {
    getObject(arg0).remove();
}
export function __wbg_remove_d0b0029ab01038e1() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).remove(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
}, arguments); }
export function __wbg_requestAnimationFrame_9ba49d36a19ca83c() { return handleError(function (arg0, arg1) {
    const ret = getObject(arg0).requestAnimationFrame(getObject(arg1));
    return ret;
}, arguments); }
export function __wbg_resolve_dbf78ab4942478b0(arg0) {
    const ret = Promise.resolve(getObject(arg0));
    return addHeapObject(ret);
}
export function __wbg_respond_d97c023ee981d1f5() { return handleError(function (arg0, arg1) {
    getObject(arg0).respond(arg1 >>> 0);
}, arguments); }
export function __wbg_revokeObjectURL_52940372bc4245cf() { return handleError(function (arg0, arg1) {
    URL.revokeObjectURL(getStringFromWasm0(arg0, arg1));
}, arguments); }
export function __wbg_scrollHeight_4e4f6d0987910b5b(arg0) {
    const ret = getObject(arg0).scrollHeight;
    return ret;
}
export function __wbg_scrollTop_960220fc942d86fa(arg0) {
    const ret = getObject(arg0).scrollTop;
    return ret;
}
export function __wbg_select_f3d8449d7b38fb10(arg0) {
    getObject(arg0).select();
}
export function __wbg_setAttribute_f75317b2b5cabfb4() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
}, arguments); }
export function __wbg_setProperty_3fe74e09d53acb0b() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
    getObject(arg0).setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
}, arguments); }
export function __wbg_setSelectionRange_b8e9440b4c74a17b() { return handleError(function (arg0, arg1, arg2) {
    getObject(arg0).setSelectionRange(arg1 >>> 0, arg2 >>> 0);
}, arguments); }
export function __wbg_setTimeout_9ef22b48643bd7de() { return handleError(function (arg0, arg1, arg2) {
    const ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
    return ret;
}, arguments); }
export function __wbg_set_2793479888329cb6() { return handleError(function (arg0, arg1, arg2) {
    const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
    return ret;
}, arguments); }
export function __wbg_set_className_fcd90345b3a090af(arg0, arg1, arg2) {
    getObject(arg0).className = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_dropEffect_81bf03e9cf54de98(arg0, arg1, arg2) {
    getObject(arg0).dropEffect = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_feb3714f475485ba(arg0, arg1, arg2) {
    getObject(arg0).set(getArrayU8FromWasm0(arg1, arg2));
}
export function __wbg_set_hidden_7efdf3958dc524e5(arg0, arg1) {
    getObject(arg0).hidden = arg1 !== 0;
}
export function __wbg_set_innerHTML_a9d3a267967b5117(arg0, arg1, arg2) {
    getObject(arg0).innerHTML = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_nodeValue_7e9217115013d873(arg0, arg1, arg2) {
    getObject(arg0).nodeValue = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_prevent_scroll_1d8392770c4be456(arg0, arg1) {
    getObject(arg0).preventScroll = arg1 !== 0;
}
export function __wbg_set_scrollTop_9abe010024623249(arg0, arg1) {
    getObject(arg0).scrollTop = arg1;
}
export function __wbg_set_textContent_0177b2a6d9293c2c(arg0, arg1, arg2) {
    getObject(arg0).textContent = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_type_609b0cea6429d7bb(arg0, arg1, arg2) {
    getObject(arg0).type = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_value_651c5a714f0a7b56(arg0, arg1, arg2) {
    getObject(arg0).value = getStringFromWasm0(arg1, arg2);
}
export function __wbg_set_value_cb7e77c93a95e491(arg0, arg1, arg2) {
    getObject(arg0).value = getStringFromWasm0(arg1, arg2);
}
export function __wbg_showModal_03450d22087f2f44() { return handleError(function (arg0) {
    getObject(arg0).showModal();
}, arguments); }
export function __wbg_size_9d1084e29743663c(arg0) {
    const ret = getObject(arg0).size;
    return ret;
}
export function __wbg_stack_575c4e0918581b9c(arg0, arg1) {
    const ret = getObject(arg1).stack;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
}
export function __wbg_static_accessor_GLOBAL_THIS_003ee833af9699fc() {
    const ret = typeof globalThis === 'undefined' ? undefined : globalThis;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_static_accessor_GLOBAL_e4293fb22cbe77da() {
    const ret = typeof global === 'undefined' ? undefined : global;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_static_accessor_SELF_53bb20457401453d() {
    const ret = typeof self === 'undefined' ? undefined : self;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_static_accessor_WINDOW_b5e0810bfecb7feb() {
    const ret = typeof window === 'undefined' ? undefined : window;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_style_f499e599ac70277f(arg0) {
    const ret = getObject(arg0).style;
    return addHeapObject(ret);
}
export function __wbg_target_e5cd9c7aa87dd8df(arg0) {
    const ret = getObject(arg0).target;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbg_text_302c9b0fbcdc0030(arg0) {
    const ret = getObject(arg0).text();
    return addHeapObject(ret);
}
export function __wbg_then_656f253a5dd7d8ea(arg0, arg1, arg2) {
    const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
}
export function __wbg_then_e3f7f2d365708be8(arg0, arg1) {
    const ret = getObject(arg0).then(getObject(arg1));
    return addHeapObject(ret);
}
export function __wbg_toggle_4ad9c24ed28f124e() { return handleError(function (arg0, arg1, arg2, arg3) {
    const ret = getObject(arg0).toggle(getStringFromWasm0(arg1, arg2), arg3 !== 0);
    return ret;
}, arguments); }
export function __wbg_type_c93f3ee8b7eb4f89(arg0, arg1) {
    const ret = getObject(arg1).type;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
}
export function __wbg_types_9d342bd82d237873(arg0) {
    const ret = getObject(arg0).types;
    return addHeapObject(ret);
}
export function __wbg_value_df0a0e29a4d182fe(arg0, arg1) {
    const ret = getObject(arg1).value;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
}
export function __wbg_value_f61036dc298faa36(arg0, arg1) {
    const ret = getObject(arg1).value;
    const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
    getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
}
export function __wbg_view_3b708d3bdaf3fc96(arg0) {
    const ret = getObject(arg0).view;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000001(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 416, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
    const ret = makeMutClosure(arg0, arg1, wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___wasm_bindgen_177ff7d6ee1925ce___JsValue______true_);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000002(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 454, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
    const ret = makeMutClosure(arg0, arg1, wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___wasm_bindgen_177ff7d6ee1925ce___JsValue__core_d941ad8722425d2e___result__Result_____wasm_bindgen_177ff7d6ee1925ce___JsError___true_);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000003(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("Event")], shim_idx: 416, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
    const ret = makeMutClosure(arg0, arg1, wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___wasm_bindgen_177ff7d6ee1925ce___JsValue______true__14);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000004(arg0, arg1) {
    // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 415, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
    const ret = makeMutClosure(arg0, arg1, wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke_______true_);
    return addHeapObject(ret);
}
export function __wbindgen_generic_0000000000000005(arg0, arg1) {
    // Cast intrinsic for `Ref(String) -> Externref`.
    const ret = getStringFromWasm0(arg0, arg1);
    return addHeapObject(ret);
}
export function __wbindgen_object_clone_ref(arg0) {
    const ret = getObject(arg0);
    return addHeapObject(ret);
}
export function __wbindgen_object_drop_ref(arg0) {
    takeObject(arg0);
}
function wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke_______true_(arg0, arg1) {
    wasm.wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke_______true_(arg0, arg1);
}

function wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___wasm_bindgen_177ff7d6ee1925ce___JsValue______true_(arg0, arg1, arg2) {
    wasm.wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___wasm_bindgen_177ff7d6ee1925ce___JsValue______true_(arg0, arg1, addHeapObject(arg2));
}

function wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___wasm_bindgen_177ff7d6ee1925ce___JsValue______true__14(arg0, arg1, arg2) {
    wasm.wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___wasm_bindgen_177ff7d6ee1925ce___JsValue______true__14(arg0, arg1, addHeapObject(arg2));
}

function wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___js_sys_e915b9bba08aaee2___Function_fn_wasm_bindgen_177ff7d6ee1925ce___JsValue_____wasm_bindgen_177ff7d6ee1925ce___sys__Undefined___js_sys_e915b9bba08aaee2___Function_fn_wasm_bindgen_177ff7d6ee1925ce___JsValue_____wasm_bindgen_177ff7d6ee1925ce___sys__Undefined_______true_(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___js_sys_e915b9bba08aaee2___Function_fn_wasm_bindgen_177ff7d6ee1925ce___JsValue_____wasm_bindgen_177ff7d6ee1925ce___sys__Undefined___js_sys_e915b9bba08aaee2___Function_fn_wasm_bindgen_177ff7d6ee1925ce___JsValue_____wasm_bindgen_177ff7d6ee1925ce___sys__Undefined_______true_(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

function wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___wasm_bindgen_177ff7d6ee1925ce___JsValue__core_d941ad8722425d2e___result__Result_____wasm_bindgen_177ff7d6ee1925ce___JsError___true_(arg0, arg1, arg2) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16) >>> 0;
        wasm.wasm_bindgen_177ff7d6ee1925ce___convert__closures_____invoke___wasm_bindgen_177ff7d6ee1925ce___JsValue__core_d941ad8722425d2e___result__Result_____wasm_bindgen_177ff7d6ee1925ce___JsError___true_(retptr, arg0, arg1, addHeapObject(arg2));
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}


const __wbindgen_enum_ReadableStreamType = ["bytes"];
const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr, 1));
const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr, 1));
const IntoUnderlyingSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsource_free(ptr, 1));

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    switch (typeof val) {
        // primitive types
        case 'number':
        case 'boolean':
        case 'undefined':
        return `${val}`;
        case 'string':
        return JSON.stringify(val);
        case 'bigint':
        return `${val}n`;
        case 'symbol':
        return val.description !== undefined
            ? `Symbol(${val.description})`
            : 'Symbol';
        case 'function':
        return typeof val.name === 'string' && val.name !== ''
            ? `Function(${val.name})`
            : 'Function';
        case 'object': {
            if (val === null) {
                return 'null';
            }
            // objects
            if (Array.isArray(val)) {
                return '[' + Array.from(val, debugString).join(', ') + ']';
            }
            // errors
            if (val instanceof Error) {
                return `${val.name}: ${val.message}\n${val.stack}`;
            }

            const toStringOutput = toString.call(val);
            // Test for built-in
            const className = /\[object ([^\]]+)\]/.exec(toStringOutput)?.[1];

            // Failed to match the standard '[object ClassName]'
            if (className === undefined) {
                return toStringOutput;
            }
            if (className === 'Object') {
                // we're a user defined class or Object
                // JSON.stringify avoids problems with cycles, and is generally much
                // easier than looping through ownProperties of `val`.
                try {
                    return 'Object(' + JSON.stringify(val) + ')';
                } catch (_) {
                    return 'Object';
                }
            }
            // TODO we could test for more things here, like `Set`s and `Map`s.
            return className;
        }
        // Per MDN: 'This list of values is exhaustive.'
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/typeof#description
    }
}

function dropObject(idx) {
    if (idx < 1028) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getObject(idx) { return heap[idx]; }

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}

let heap = new Array(1024).fill(undefined);
heap.push(undefined, null, true, false);

let heap_next = heap.length;

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(bytes) {
    numBytesDecoded += bytes.length;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
        cachedTextDecoder.decode();
        numBytesDecoded = bytes.length;
    }
    return cachedTextDecoder.decode(bytes);
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;


let wasm;
export function __wbg_set_wasm(val) {
    wasm = val;
}
