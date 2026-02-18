export type GridWindow = {
  rowStart: number;
  rowCount: number;
  colStart: number;
  colCount: number;
};

export type GridTile = GridWindow & {
  rowTile: number;
  colTile: number;
  key: string;
};

export function normalizeWindow(
  input: GridWindow,
  maxRows: number,
  maxCols: number
): GridWindow {
  const rowStart = Math.max(0, Math.min(input.rowStart, Math.max(0, maxRows)));
  const colStart = Math.max(0, Math.min(input.colStart, Math.max(0, maxCols)));
  const rowCount = Math.max(0, Math.min(input.rowCount, Math.max(0, maxRows - rowStart)));
  const colCount = Math.max(0, Math.min(input.colCount, Math.max(0, maxCols - colStart)));
  return { rowStart, rowCount, colStart, colCount };
}

export function expandWindow(
  input: GridWindow,
  maxRows: number,
  maxCols: number,
  prefetchRows: number,
  prefetchCols: number
): GridWindow {
  const expanded = {
    rowStart: input.rowStart - prefetchRows,
    rowCount: input.rowCount + prefetchRows * 2,
    colStart: input.colStart - prefetchCols,
    colCount: input.colCount + prefetchCols * 2
  };
  return normalizeWindow(expanded, maxRows, maxCols);
}

export function tileKey(rowTile: number, colTile: number): string {
  return `${rowTile}:${colTile}`;
}

export function tilesForWindow(
  window: GridWindow,
  maxRows: number,
  maxCols: number,
  tileRows: number,
  tileCols: number
): GridTile[] {
  const normalized = normalizeWindow(window, maxRows, maxCols);
  if (normalized.rowCount <= 0 || normalized.colCount <= 0) return [];

  const rowEndExclusive = normalized.rowStart + normalized.rowCount;
  const colEndExclusive = normalized.colStart + normalized.colCount;

  const startRowTile = Math.floor(normalized.rowStart / tileRows);
  const endRowTile = Math.floor((rowEndExclusive - 1) / tileRows);
  const startColTile = Math.floor(normalized.colStart / tileCols);
  const endColTile = Math.floor((colEndExclusive - 1) / tileCols);

  const out: GridTile[] = [];
  for (let rowTile = startRowTile; rowTile <= endRowTile; rowTile++) {
    for (let colTile = startColTile; colTile <= endColTile; colTile++) {
      const rowStart = rowTile * tileRows;
      const colStart = colTile * tileCols;
      const tile = normalizeWindow(
        {
          rowStart,
          rowCount: tileRows,
          colStart,
          colCount: tileCols
        },
        maxRows,
        maxCols
      );
      out.push({
        ...tile,
        rowTile,
        colTile,
        key: tileKey(rowTile, colTile)
      });
    }
  }
  return out;
}

