import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatSize(bytes?: number): string {
  if (!bytes) return "-";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** Parse an RFC3339 date string (from Rust backend) and format as local datetime.
 *  Input:  "2026-05-18T10:30:00+00:00"
 *  Output: "2026-05-18 18:30:00" (in China UTC+8) */
export function formatDateTime(rfc3339?: string): string {
  if (!rfc3339) return "-";
  try {
    const d = new Date(rfc3339);
    if (isNaN(d.getTime())) return rfc3339.slice(0, 19).replace("T", " ");
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
  } catch {
    return rfc3339.slice(0, 19).replace("T", " ");
  }
}

/** Parse an RFC3339 date string and format as local date only.
 *  Input:  "2026-05-18T10:30:00+00:00"
 *  Output: "2026-05-18" */
export function formatDate(rfc3339?: string): string {
  if (!rfc3339) return "-";
  try {
    const d = new Date(rfc3339);
    if (isNaN(d.getTime())) return rfc3339.slice(0, 10);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  } catch {
    return rfc3339.slice(0, 10);
  }
}
