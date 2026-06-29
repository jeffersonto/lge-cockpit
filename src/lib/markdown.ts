/**
 * GFM tables require contiguous rows; Claude CLI sometimes inserts a blank
 * line between rows when serializing ADF tables. Drop those blank lines so
 * react-markdown + remark-gfm render the table as a single block.
 */
export function fixMarkdownTables(md: string): string {
  const lines = md.split("\n");
  const result: string[] = [];
  let inTable = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    const isTableRow = trimmed.startsWith("|") && trimmed.endsWith("|");
    const isSeparator = /^\|[-:\s|]+\|$/.test(trimmed);

    if (isTableRow || isSeparator) {
      inTable = true;
      result.push(line);
    } else if (inTable && trimmed === "") {
      let nextNonBlank = i + 1;
      while (nextNonBlank < lines.length && lines[nextNonBlank].trim() === "") {
        nextNonBlank++;
      }
      if (
        nextNonBlank < lines.length &&
        lines[nextNonBlank].trim().startsWith("|")
      ) {
        continue;
      } else {
        inTable = false;
        result.push(line);
      }
    } else {
      inTable = false;
      result.push(line);
    }
  }

  return result.join("\n");
}
