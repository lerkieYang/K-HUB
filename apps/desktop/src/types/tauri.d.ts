declare global {
  interface Window {
    __TAURI__?: {
      convertFileSrc: (src: string, protocol: string) => string;
      invoke: (cmd: string, args?: Record<string, any>) => Promise<any>;
      shell: {
        open: (url: string) => Promise<void>;
      };
    };
  }
}

export {};
