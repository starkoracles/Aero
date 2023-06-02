declare module '@marco_ciaramella/sha256-gpu' {
    export function sha256_gpu(messages: Array<Uint8Array>): Promise<Array<Uint8Array>>;
}