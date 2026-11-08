export interface SearchQuery {
  text: string;
  appOnly: boolean;
  fileOnly: boolean;
  recentOnly: boolean;
  type: string | null;
  folder: string | null;
}

export interface FileFilterInput {
  path: string;
  extension: string;
  parent: string;
}

const EMPTY_QUERY: SearchQuery = {
  text: "",
  appOnly: false,
  fileOnly: false,
  recentOnly: false,
  type: null,
  folder: null,
};

export function parseSearchQuery(raw: string): SearchQuery {
  const query = { ...EMPTY_QUERY };
  const words: string[] = [];

  for (const token of raw.trim().split(/\s+/u).filter(Boolean)) {
    const lower = token.toLowerCase();
    if (lower === "app:") {
      query.appOnly = true;
    } else if (lower.startsWith("app:") && lower.length > 4) {
      query.appOnly = true;
      words.push(token.slice(4));
    } else if (lower === "file:") {
      query.fileOnly = true;
    } else if (lower.startsWith("file:")) {
      const value = token.slice(5);
      query.fileOnly = true;
      if (value.startsWith("folder:")) {
        const folder = value.slice(7).toLowerCase().replace(/^\./u, "");
        if (folder) {
          query.folder = folder;
          query.fileOnly = true;
        } else {
          words.push(value);
        }
      } else if (value) {
        words.push(value);
      }
    } else if (lower === "recent:") {
      query.recentOnly = true;
    } else if (lower.startsWith("type:") && lower.length > 5) {
      query.type = lower.slice(5).replace(/^\./u, "");
      query.fileOnly = true;
    } else if (lower.startsWith("folder:") && lower.length > 7) {
      query.folder = lower.slice(7);
      query.fileOnly = true;
    } else {
      words.push(token);
    }
  }

  query.text = words.join(" ");
  return query;
}

export function hasSearchFilters(query: SearchQuery): boolean {
  return (
    query.appOnly ||
    query.fileOnly ||
    query.recentOnly ||
    query.type !== null ||
    query.folder !== null
  );
}

const FOLDER_ALIASES: Record<string, string[]> = {
  desktop: ["desktop", "桌面"],
  documents: ["documents", "document", "文档"],
  downloads: ["downloads", "download", "下载"],
  pictures: ["pictures", "picture", "images", "图片"],
};

function folderMatches(file: FileFilterInput, requested: string): boolean {
  const normalized = requested.toLowerCase();
  const names = new Set(
    [file.parent]
      .flatMap((value) => value.split(/[\\/]/u))
      .map((value) => value.toLowerCase()),
  );
  const aliases = FOLDER_ALIASES[normalized] ?? [normalized];
  return aliases.some((alias) => names.has(alias));
}

export function matchesFileFilters(
  file: FileFilterInput,
  filters: Pick<SearchQuery, "type" | "folder">,
): boolean {
  if (
    filters.type &&
    file.extension.toLowerCase() !== filters.type.toLowerCase()
  ) {
    return false;
  }
  if (filters.folder && !folderMatches(file, filters.folder)) {
    return false;
  }
  return true;
}
