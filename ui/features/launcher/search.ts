import type { ToolDefinition } from '../../bindings/types';

export function searchTools(
  tools: readonly ToolDefinition[],
  query: string,
  favorites: readonly string[],
  onlyFavorites: boolean,
): ToolDefinition[] {
  const terms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  return tools.filter((tool) => {
    if (onlyFavorites && !favorites.includes(tool.id)) return false;
    const searchable = [tool.id, tool.title, tool.description, tool.category, ...tool.keywords]
      .join(' ')
      .toLocaleLowerCase();
    return terms.every((term) => searchable.includes(term));
  });
}
