import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatRelativeTime(unixSec: number): string {
  const diff = Math.floor(Date.now() / 1000) - unixSec;
  if (diff < 60)    return `${diff} 秒前`;
  if (diff < 3600)  return `${Math.floor(diff / 60)} 分前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} 時間前`;
  return `${Math.floor(diff / 86400)} 日前`;
}
