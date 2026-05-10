import React, { useState, useEffect, useCallback, createContext, useContext, useRef } from 'react';

// Toast types
type ToastType = 'success' | 'error' | 'warning' | 'info';

interface Toast {
  id: string;
  type: ToastType;
  title?: string;
  message: string;
  duration?: number;
}

interface ConfirmDialog {
  id: string;
  title: string;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmText?: string;
  cancelText?: string;
  danger?: boolean;
}

interface ToastContextType {
  toast: (message: string, type?: ToastType, title?: string, duration?: number) => void;
  success: (message: string, title?: string) => void;
  error: (message: string, title?: string) => void;
  warning: (message: string, title?: string) => void;
  info: (message: string, title?: string) => void;
  confirm: (message: string, title?: string, options?: { confirmText?: string; cancelText?: string; danger?: boolean }) => Promise<boolean>;
}

const ToastContext = createContext<ToastContextType | null>(null);

export function useToast(): ToastContextType {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error('useToast must be used within ToastProvider');
  return ctx;
}

// Toast item component
function ToastItem({ toast, onRemove }: { toast: Toast; onRemove: (id: string) => void }) {
  const [exiting, setExiting] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => {
      setExiting(true);
      setTimeout(() => onRemove(toast.id), 300);
    }, toast.duration || 3000);
    return () => clearTimeout(timer);
  }, [toast.id, toast.duration, onRemove]);

  const icons: Record<ToastType, string> = {
    success: '✅',
    error: '🔴',
    warning: '⚠️',
    info: 'ℹ️',
  };

  const colors: Record<ToastType, string> = {
    success: 'bg-emerald-50 border-emerald-200 text-emerald-800',
    error: 'bg-red-50 border-red-200 text-red-800',
    warning: 'bg-amber-50 border-amber-200 text-amber-800',
    info: 'bg-blue-50 border-blue-200 text-blue-800',
  };

  return (
    <div
      className={`flex items-start gap-3 px-4 py-3 rounded-xl border shadow-lg backdrop-blur-sm transition-all duration-300 ${colors[toast.type]} ${exiting ? 'opacity-0 translate-x-8' : 'opacity-100 translate-x-0'}`}
    >
      <div className="min-w-0 flex-1">
        {toast.title && <p className="font-semibold text-sm">{toast.title}</p>}
        <p className="text-sm">{toast.message}</p>
      </div>
      <button onClick={() => { setExiting(true); setTimeout(() => onRemove(toast.id), 300); }} className="text-gray-400 hover:text-gray-600 flex-shrink-0">✕</button>
    </div>
  );
}

// Confirm dialog component
function ConfirmDialogItem({ dialog }: { dialog: ConfirmDialog }) {
  const [exiting, setExiting] = useState(false);

  const handleConfirm = () => {
    setExiting(true);
    setTimeout(() => dialog.onConfirm(), 200);
  };

  const handleCancel = () => {
    setExiting(true);
    setTimeout(() => dialog.onCancel(), 200);
  };

  return (
    <div className={`fixed inset-0 z-[9999] flex items-center justify-center p-4 transition-all duration-200 ${exiting ? 'opacity-0' : 'opacity-100'}`}>
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" onClick={handleCancel} />
      <div className={`relative bg-white rounded-2xl shadow-2xl max-w-md w-full overflow-hidden transition-all duration-200 ${exiting ? 'scale-95' : 'scale-100'}`}>
        <div className="p-5">
          <h3 className="text-lg font-semibold text-gray-900 mb-2">{dialog.title}</h3>
          <p className="text-sm text-gray-600">{dialog.message}</p>
        </div>
        <div className="flex gap-2 justify-end px-5 py-4 border-t bg-gray-50">
          <button onClick={handleCancel} className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-xl hover:bg-gray-50 transition">
            {dialog.cancelText || '取消'}
          </button>
          <button onClick={handleConfirm} className={`px-4 py-2 text-sm font-medium text-white rounded-xl transition ${dialog.danger ? 'bg-red-600 hover:bg-red-700' : 'bg-indigo-600 hover:bg-indigo-700'}`}>
            {dialog.confirmText || '确认'}
          </button>
        </div>
      </div>
    </div>
  );
}

// Provider
export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [confirms, setConfirms] = useState<ConfirmDialog[]>([]);
  const counterRef = useRef(0);

  const removeToast = useCallback((id: string) => {
    setToasts(prev => prev.filter(t => t.id !== id));
  }, []);

  const toast = useCallback((message: string, type: ToastType = 'info', title?: string, duration?: number) => {
    const id = `toast-${++counterRef.current}`;
    setToasts(prev => [...prev, { id, type, title, message, duration }]);
  }, []);

  const success = useCallback((message: string, title?: string) => toast(message, 'success', title), [toast]);
  const error = useCallback((message: string, title?: string) => toast(message, 'error', title), [toast]);
  const warning = useCallback((message: string, title?: string) => toast(message, 'warning', title), [toast]);
  const info = useCallback((message: string, title?: string) => toast(message, 'info', title), [toast]);

  const confirm = useCallback((message: string, title = '确认', options?: { confirmText?: string; cancelText?: string; danger?: boolean }): Promise<boolean> => {
    return new Promise((resolve) => {
      const id = `confirm-${++counterRef.current}`;
      setConfirms(prev => [...prev, {
        id, title, message,
        confirmText: options?.confirmText,
        cancelText: options?.cancelText,
        danger: options?.danger,
        onConfirm: () => { setConfirms(prev => prev.filter(c => c.id !== id)); resolve(true); },
        onCancel: () => { setConfirms(prev => prev.filter(c => c.id !== id)); resolve(false); },
      }]);
    });
  }, []);

  return (
    <ToastContext.Provider value={{ toast, success, error, warning, info, confirm }}>
      {children}
      {/* Toast container - top right */}
      <div className="fixed top-4 right-4 z-[9998] flex flex-col gap-2 w-80">
        {toasts.map(t => (
          <ToastItem key={t.id} toast={t} onRemove={removeToast} />
        ))}
      </div>
      {/* Confirm dialogs */}
      {confirms.map(c => (
        <ConfirmDialogItem key={c.id} dialog={c} />
      ))}
    </ToastContext.Provider>
  );
}
