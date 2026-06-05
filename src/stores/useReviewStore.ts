import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { ReviewItem } from "@/types/review";

interface ReviewSummary {
  id: string;
  task_id: string;
  kb_id: string;
  status: string;
  summary: string;
  risk_level: string;
  created_at: string;
  items: ReviewItem[];
}

interface ReviewState {
  pendingCount: number;
  pendingItems: ReviewItem[];
  reviews: ReviewSummary[];
  loading: boolean;
  error: string | null;
  processingIds: Set<string>;
  loadPendingReviews: (kbId: string) => Promise<void>;
  acceptItem: (itemId: string, kbId: string, kbPath: string) => Promise<void>;
  rejectItem: (itemId: string) => Promise<void>;
  clearError: () => void;
}

export const useReviewStore = create<ReviewState>((set, get) => ({
  pendingCount: 0,
  pendingItems: [],
  reviews: [],
  loading: false,
  error: null,
  processingIds: new Set(),

  clearError: () => set({ error: null }),

  loadPendingReviews: async (kbId: string) => {
    set({ loading: true, error: null });
    try {
      const rawReviews = await invoke<ReviewSummary[]>("get_pending_reviews", { kbId });
      // Flatten nested items with status='pending' only (skip already accepted/rejected)
      const flatItems: ReviewItem[] = [];
      for (const review of rawReviews) {
        for (const item of review.items) {
          if (item.status === "pending" || item.status === "accepted") {
            flatItems.push({ ...item, task_id: item.task_id || review.task_id });
          }
        }
      }
      set({ reviews: rawReviews, pendingItems: flatItems, pendingCount: flatItems.length, loading: false });
    } catch (e) {
      console.error("加载审阅列表失败:", e);
      set({ loading: false, error: String(e), pendingItems: [] });
    }
  },

  acceptItem: async (itemId: string, kbId: string, kbPath: string) => {
    if (get().processingIds.has(itemId)) return;
    set((state) => ({ processingIds: new Set(state.processingIds).add(itemId) }));
    try {
      await invoke("accept_review_item", { itemId, kbId, kbPath });
      const { pendingItems } = get();
      const next = pendingItems.filter((i) => i.id !== itemId);
      set({ pendingItems: next, pendingCount: next.length });
    } catch (e) {
      console.error("接受审阅项失败:", e);
      throw e;
    } finally {
      set((state) => {
        const next = new Set(state.processingIds);
        next.delete(itemId);
        return { processingIds: next };
      });
    }
  },

  rejectItem: async (itemId: string) => {
    if (get().processingIds.has(itemId)) return;
    set((state) => ({ processingIds: new Set(state.processingIds).add(itemId) }));
    try {
      await invoke("reject_review_item", { itemId });
      const { pendingItems } = get();
      const next = pendingItems.filter((i) => i.id !== itemId);
      set({ pendingItems: next, pendingCount: next.length });
    } catch (e) {
      console.error("拒绝审阅项失败:", e);
      throw e;
    } finally {
      set((state) => {
        const next = new Set(state.processingIds);
        next.delete(itemId);
        return { processingIds: next };
      });
    }
  },
}));
