import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { FileScanResult } from "@/types/source";

export interface FileTreeNode {
  name: string;
  relative_path: string;
  file_type: string;
  file_size: number;
  modified_at: string;
  is_directory: boolean;
  children?: FileTreeNode[];
  record_type: string;
  linked_record_id: string;
  status: string;
  // Backward-compat aliases used by FileBrowserPage / FileTree components
  path?: string;
  is_dir?: boolean;
  extension?: string;
  size?: number;
}

export type SortMode = "name" | "modified" | "type";

interface FileTreeState {
  files: FileTreeNode[];
  expandedFolders: Set<string>;
  selectedFile: FileTreeNode | null;
  loading: boolean;
  error: string | null;
  _kbId: string | null;
  _kbPath: string | null;
  sortBy: SortMode;

  loadFileTree: (kbId: string, kbPath: string) => Promise<void>;
  toggleFolder: (path: string) => void;
  selectFile: (node: FileTreeNode | null) => void;
  refreshTree: () => Promise<void>;
  setSortBy: (sortBy: SortMode) => void;
  expandAll: () => void;
  collapseAll: () => void;
  getTreeStats: () => { fileCount: number; folderCount: number };
}

function countTree(nodes: FileTreeNode[]): { fileCount: number; folderCount: number } {
  let fileCount = 0;
  let folderCount = 0;
  if (!Array.isArray(nodes)) return { fileCount, folderCount };
  for (const node of nodes) {
    if (node.is_directory || node.is_dir) {
      folderCount += 1;
      if (node.children) {
        const sub = countTree(node.children);
        fileCount += sub.fileCount;
        folderCount += sub.folderCount;
      }
    } else {
      fileCount += 1;
    }
  }
  return { fileCount, folderCount };
}

function collectFolderPaths(nodes: FileTreeNode[]): string[] {
  const paths: string[] = [];
  if (!Array.isArray(nodes)) return paths;
  for (const node of nodes) {
    if (node.is_directory || node.is_dir) {
      paths.push(node.relative_path);
      if (node.children) {
        paths.push(...collectFolderPaths(node.children));
      }
    }
  }
  return paths;
}

export const useFileTreeStore = create<FileTreeState>((set, get) => ({
  files: [],
  expandedFolders: new Set(),
  selectedFile: null,
  loading: false,
  error: null,
  _kbId: null,
  _kbPath: null,
  sortBy: "name",

  loadFileTree: async (kbId: string, kbPath: string) => {
    set({ loading: true, error: null, _kbId: kbId, _kbPath: kbPath });
    try {
      const result = await invoke<FileScanResult>("get_file_tree", { kbId, kbPath });
      const tree = Array.isArray(result?.root?.children) ? result.root.children : [];
      set({ files: tree, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  toggleFolder: (path: string) => {
    const { expandedFolders } = get();
    const next = new Set(expandedFolders);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    set({ expandedFolders: next });
  },

  selectFile: (node: FileTreeNode | null) => {
    set({ selectedFile: node });
  },

  refreshTree: async () => {
    const { _kbId, _kbPath } = get();
    if (!_kbId || !_kbPath) {
      set({ error: "知识库未初始化" });
      return;
    }
    set({ loading: true, error: null });
    try {
      const result = await invoke<FileScanResult>("get_file_tree", { kbId: _kbId, kbPath: _kbPath });
      const tree = Array.isArray(result?.root?.children) ? result.root.children : [];
      set({ files: tree, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  setSortBy: (sortBy: SortMode) => {
    set({ sortBy });
  },

  expandAll: () => {
    set({ expandedFolders: new Set(collectFolderPaths(get().files)) });
  },

  collapseAll: () => {
    set({ expandedFolders: new Set() });
  },

  getTreeStats: () => {
    return countTree(get().files);
  },
}));
