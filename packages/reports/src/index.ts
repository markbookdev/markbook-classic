export type ReportModel = {
  title: string;
  body: string;
};

export function renderReportHtml(model: ReportModel): string {
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4; margin: 16mm; }
      body { font-family: Arial, sans-serif; color: #111; }
      h1 { font-size: 18px; margin: 0 0 8px 0; }
      .body { font-size: 12px; white-space: pre-wrap; }
    </style>
  </head>
  <body>
    <h1>${escapeHtml(model.title)}</h1>
    <div class="body">${escapeHtml(model.body)}</div>
  </body>
</html>`;
}

export type MarkSetGridReportModel = {
  class: { id: string; name: string };
  markSet: { id: string; code: string; description: string };
  students: Array<{ id: string; displayName: string; sortOrder: number; active: boolean }>;
  assessments: Array<{
    id: string;
    idx: number;
    date: string | null;
    categoryName: string | null;
    title: string;
    weight: number | null;
    outOf: number | null;
  }>;
  assessmentAverages: Array<{
    assessmentId: string;
    idx: number;
    avgRaw: number;
    avgPercent: number;
    scoredCount: number;
    zeroCount: number;
    noMarkCount: number;
  }>;
  // [row][col] raw score semantics:
  // - null => No Mark (blank)
  // - 0 => Zero (counts as 0)
  // - >0 => scored
  cells: Array<Array<number | null>>;
  filters?: {
    term: number | null;
    categoryName: string | null;
    typesMask: number | null;
  };
  studentScope?: "all" | "active" | "valid";
};

export function renderMarkSetGridReportHtml(model: MarkSetGridReportModel): string {
  const generatedAt = new Date().toLocaleString();

  const headers = model.assessments.map((a) => {
      const date = a.date ? `<div class="th-sub">${escapeHtml(a.date)}</div>` : "";
      const cat = a.categoryName ? `<div class="th-sub">${escapeHtml(a.categoryName)}</div>` : "";
      const outOf = a.outOf != null ? `(${a.outOf})` : "";
      return `<th class="assess" title="${escapeHtml(a.title)}">${escapeHtml(
        a.title
      )} ${escapeHtml(outOf)}${date}${cat}</th>`;
    }).join("");

  const bodyRows = model.students.map((s, rIdx) => {
      const row = model.cells[rIdx] ?? [];
      const tds = model.assessments.map((_a, cIdx) => {
          const v = row[cIdx] ?? null;
          const txt = v == null ? "" : String(v);
          return `<td class="mark">${escapeHtml(txt)}</td>`;
        }).join("");
      return `<tr><td class="student">${escapeHtml(s.displayName)}</td>${tds}</tr>`;
    }).join("");

  const averages = model.assessmentAverages ?? [];
  const footerRow =
    averages.length === model.assessments.length
      ? `<tfoot><tr><td class="student">Avg (active)</td>${averages
          .map((a) => {
            const denom = (a.scoredCount ?? 0) + (a.zeroCount ?? 0);
            const txt = denom > 0 ? a.avgRaw.toFixed(1) : "";
            return `<td class="mark">${escapeHtml(txt)}</td>`;
          })
          .join("")}</tr></tfoot>`
      : "";

  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4 landscape; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, \"Helvetica Neue\", Helvetica, Arial, sans-serif; color: #111; }
      .header { display: flex; justify-content: space-between; align-items: flex-end; gap: 12px; margin-bottom: 10px; }
      .title { font-size: 16px; font-weight: 700; margin: 0; }
      .meta { font-size: 11px; color: #444; text-align: right; line-height: 1.25; }
      .meta div { white-space: nowrap; }

      table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      thead { display: table-header-group; }
      tfoot { display: table-footer-group; }
      tr { break-inside: avoid; page-break-inside: avoid; }
      th, td { border: 1px solid #ccc; padding: 3px 4px; font-size: 10px; vertical-align: top; }
      th { background: #f6f6f6; font-weight: 700; }
      tfoot td { background: #fafafa; font-weight: 700; }

      th.assess { white-space: normal; line-height: 1.15; }
      .th-sub { font-weight: 400; color: #555; }

      td.student { font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
      td.mark { text-align: right; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

      col.student { width: 52mm; }
      col.assess { width: 14mm; }
    </style>
  </head>
  <body>
    <div class="header">
      <div>
        <div class="title">Mark Set Grid</div>
        <div class="meta" style="text-align:left">
          <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
          <div><strong>Mark Set:</strong> ${escapeHtml(model.markSet.code)}: ${escapeHtml(
    model.markSet.description
  )}</div>
          <div><strong>Scope:</strong> ${escapeHtml(reportStudentScopeLabel(model.studentScope))}</div>
          <div><strong>Filters:</strong> ${escapeHtml(reportFiltersLabel(model.filters))}</div>
        </div>
      </div>
      <div class="meta">
        <div>${escapeHtml(generatedAt)}</div>
      </div>
    </div>

    <table>
      <colgroup>
        <col class="student" />
        ${model.assessments.map(() => `<col class=\"assess\" />`).join("")}
      </colgroup>
      <thead>
        <tr>
          <th>Student</th>
          ${headers}
        </tr>
      </thead>
      ${footerRow}
      <tbody>
        ${bodyRows}
      </tbody>
    </table>
  </body>
</html>`;
}

export type MarkSetSummaryReportModel = {
  class: { id: string; name: string };
  markSet: { id: string; code: string; description: string };
  settings: {
    fullCode: string | null;
    room: string | null;
    day: string | null;
    period: string | null;
    weightMethod: number;
    calcMethod: number;
  };
  filters: {
    term: number | null;
    categoryName: string | null;
    typesMask: number | null;
  };
  studentScope?: "all" | "active" | "valid";
  categories: Array<{ name: string; weight: number; sortOrder: number }>;
  assessments: Array<{
    assessmentId: string;
    idx: number;
    date: string | null;
    categoryName: string | null;
    title: string;
    term: number | null;
    legacyType: number | null;
    weight: number;
    outOf: number;
  }>;
  perAssessment: Array<{
    assessmentId: string;
    idx: number;
    date: string | null;
    categoryName: string | null;
    title: string;
    outOf: number;
    avgRaw: number;
    avgPercent: number;
    medianPercent: number;
    scoredCount: number;
    zeroCount: number;
    noMarkCount: number;
  }>;
  perCategory: Array<{
    name: string;
    weight: number;
    sortOrder: number | null;
    classAvg: number;
    studentCount: number;
    assessmentCount: number;
  }>;
  perStudent: Array<{
    studentId: string;
    displayName: string;
    sortOrder: number;
    active: boolean;
    finalMark: number | null;
    noMarkCount: number;
    zeroCount: number;
    scoredCount: number;
  }>;
};

function methodLabel(weightMethod: number): string {
  if (weightMethod === 0) return "Entry weighting";
  if (weightMethod === 1) return "Category weighting";
  if (weightMethod === 2) return "Equal weighting";
  return `Method ${weightMethod}`;
}

function reportStudentScopeLabel(scope?: "all" | "active" | "valid"): string {
  if (scope === "active") return "Active students";
  if (scope === "valid") return "Valid for mark set";
  return "All students";
}

function reportFiltersLabel(filters?: {
  term: number | null;
  categoryName: string | null;
  typesMask: number | null;
}): string {
  if (!filters) return "ALL";
  const bits: string[] = [];
  bits.push(filters.term == null ? "Term: ALL" : `Term: ${filters.term}`);
  bits.push(
    filters.categoryName == null ? "Category: ALL" : `Category: ${filters.categoryName}`
  );
  bits.push(filters.typesMask == null ? "Types: ALL" : `Types mask: ${filters.typesMask}`);
  return bits.join(", ");
}

export function renderMarkSetSummaryReportHtml(model: MarkSetSummaryReportModel): string {
  const generatedAt = new Date().toLocaleString();

  const studentRows = model.perStudent
    .map((s) => {
      const mark = s.finalMark == null ? "" : s.finalMark.toFixed(1);
      return `<tr>
        <td class="left">${escapeHtml(s.displayName)}</td>
        <td class="num">${mark}</td>
        <td class="num">${s.scoredCount}</td>
        <td class="num">${s.zeroCount}</td>
        <td class="num">${s.noMarkCount}</td>
      </tr>`;
    })
    .join("");

  const assessmentRows = model.perAssessment
    .map(
      (a) => `<tr>
      <td class="left">${escapeHtml(a.title)}</td>
      <td>${escapeHtml(a.categoryName ?? "")}</td>
      <td class="num">${a.outOf.toFixed(1)}</td>
      <td class="num">${a.avgRaw.toFixed(1)}</td>
      <td class="num">${a.avgPercent.toFixed(1)}</td>
      <td class="num">${a.medianPercent.toFixed(1)}</td>
      <td class="num">${a.scoredCount}</td>
      <td class="num">${a.zeroCount}</td>
      <td class="num">${a.noMarkCount}</td>
    </tr>`
    )
    .join("");

  const categoryRows = model.perCategory
    .map(
      (c) => `<tr>
      <td class="left">${escapeHtml(c.name)}</td>
      <td class="num">${c.weight.toFixed(1)}</td>
      <td class="num">${c.classAvg.toFixed(1)}</td>
      <td class="num">${c.studentCount}</td>
      <td class="num">${c.assessmentCount}</td>
    </tr>`
    )
    .join("");

  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      h2 { margin: 14px 0 6px 0; font-size: 14px; }
      .meta { font-size: 11px; color: #444; line-height: 1.3; }
      .top { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 8px; }
      table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      thead { display: table-header-group; }
      tfoot { display: table-footer-group; }
      tr { break-inside: avoid; page-break-inside: avoid; }
      th, td { border: 1px solid #ccc; font-size: 10px; padding: 3px 4px; }
      th { background: #f6f6f6; }
      td.left, th.left { text-align: left; }
      td.num, th.num { text-align: right; }
      .break { page-break-before: always; }
      .summary-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin-top: 8px; }
      .card { border: 1px solid #ddd; border-radius: 8px; padding: 8px; }
      .muted { color: #666; font-size: 11px; }
    </style>
  </head>
  <body>
    <div class="top">
      <div>
        <h1>Mark Set Summary</h1>
        <div class="meta">
          <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
          <div><strong>Mark Set:</strong> ${escapeHtml(model.markSet.code)}: ${escapeHtml(
    model.markSet.description
  )}</div>
          <div><strong>Scope:</strong> ${escapeHtml(reportStudentScopeLabel(model.studentScope))}</div>
          <div><strong>Filters:</strong> ${escapeHtml(reportFiltersLabel(model.filters))}</div>
        </div>
      </div>
      <div class="meta">
        <div>${escapeHtml(generatedAt)}</div>
      </div>
    </div>

    <div class="summary-grid">
      <div class="card">
        <div><strong>Weight Method:</strong> ${escapeHtml(methodLabel(model.settings.weightMethod))}</div>
        <div><strong>Calc Method:</strong> ${model.settings.calcMethod}</div>
      </div>
      <div class="card">
        <div><strong>Room/Day/Period:</strong> ${escapeHtml(
          [model.settings.room, model.settings.day, model.settings.period]
            .filter(Boolean)
            .join(" / ")
        )}</div>
        <div><strong>Full Code:</strong> ${escapeHtml(model.settings.fullCode ?? "")}</div>
      </div>
    </div>

    <h2>Category Summary</h2>
    <table>
      <thead>
        <tr>
          <th class="left">Category</th>
          <th class="num">Weight</th>
          <th class="num">Class Avg %</th>
          <th class="num">Students</th>
          <th class="num">Assessments</th>
        </tr>
      </thead>
      <tbody>${categoryRows}</tbody>
    </table>

    <h2>Assessment Stats</h2>
    <table>
      <thead>
        <tr>
          <th class="left">Assessment</th>
          <th class="left">Category</th>
          <th class="num">Out Of</th>
          <th class="num">Avg Raw</th>
          <th class="num">Avg %</th>
          <th class="num">Median %</th>
          <th class="num">Scored</th>
          <th class="num">Zero</th>
          <th class="num">No Mark</th>
        </tr>
      </thead>
      <tbody>${assessmentRows}</tbody>
    </table>

    <div class="break"></div>
    <h2>Student Final Marks</h2>
    <div class="muted">No Mark entries are excluded; Zero entries count as 0.</div>
    <table>
      <thead>
        <tr>
          <th class="left">Student</th>
          <th class="num">Final %</th>
          <th class="num">Scored</th>
          <th class="num">Zero</th>
          <th class="num">No Mark</th>
        </tr>
      </thead>
      <tbody>${studentRows}</tbody>
    </table>
  </body>
</html>`;
}

export type CategoryAnalysisReportModel = {
  class: { id: string; name: string };
  markSet: { id: string; code: string; description: string };
  settings: {
    fullCode: string | null;
    room: string | null;
    day: string | null;
    period: string | null;
    weightMethod: number;
    calcMethod: number;
  };
  filters: {
    term: number | null;
    categoryName: string | null;
    typesMask: number | null;
  };
  studentScope?: "all" | "active" | "valid";
  categories: Array<{ name: string; weight: number; sortOrder: number }>;
  perCategory: Array<{
    name: string;
    weight: number;
    sortOrder: number | null;
    classAvg: number;
    studentCount: number;
    assessmentCount: number;
  }>;
  perAssessment: Array<{
    assessmentId: string;
    idx: number;
    date: string | null;
    categoryName: string | null;
    title: string;
    outOf: number;
    avgRaw: number;
    avgPercent: number;
    medianPercent: number;
    scoredCount: number;
    zeroCount: number;
    noMarkCount: number;
  }>;
};

export function renderCategoryAnalysisReportHtml(model: CategoryAnalysisReportModel): string {
  const generatedAt = new Date().toLocaleString();
  const perCategoryRows = model.perCategory
    .map(
      (c) => `<tr>
      <td class="left">${escapeHtml(c.name)}</td>
      <td class="num">${c.weight.toFixed(1)}</td>
      <td class="num">${c.classAvg.toFixed(1)}</td>
      <td class="num">${c.studentCount}</td>
      <td class="num">${c.assessmentCount}</td>
    </tr>`
    )
    .join("");
  const perAssessmentRows = model.perAssessment
    .map(
      (a) => `<tr>
      <td class="left">${escapeHtml(a.title)}</td>
      <td class="left">${escapeHtml(a.categoryName ?? "")}</td>
      <td class="num">${a.outOf.toFixed(1)}</td>
      <td class="num">${a.avgRaw.toFixed(1)}</td>
      <td class="num">${a.avgPercent.toFixed(1)}</td>
      <td class="num">${a.medianPercent.toFixed(1)}</td>
      <td class="num">${a.scoredCount}</td>
      <td class="num">${a.zeroCount}</td>
      <td class="num">${a.noMarkCount}</td>
    </tr>`
    )
    .join("");

  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      h2 { margin: 14px 0 6px 0; font-size: 14px; }
      .meta { font-size: 11px; color: #444; line-height: 1.3; }
      .top { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 8px; }
      table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      thead { display: table-header-group; }
      tfoot { display: table-footer-group; }
      tr { break-inside: avoid; page-break-inside: avoid; }
      th, td { border: 1px solid #ccc; font-size: 10px; padding: 3px 4px; }
      th { background: #f6f6f6; }
      td.left, th.left { text-align: left; }
      td.num, th.num { text-align: right; }
      .break { page-break-before: always; }
    </style>
  </head>
  <body>
    <div class="top">
      <div>
        <h1>Category Analysis</h1>
        <div class="meta">
          <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
          <div><strong>Mark Set:</strong> ${escapeHtml(model.markSet.code)}: ${escapeHtml(
    model.markSet.description
  )}</div>
          <div><strong>Scope:</strong> ${escapeHtml(reportStudentScopeLabel(model.studentScope))}</div>
          <div><strong>Filters:</strong> ${escapeHtml(reportFiltersLabel(model.filters))}</div>
        </div>
      </div>
      <div class="meta">${escapeHtml(generatedAt)}</div>
    </div>

    <h2>Category Summary</h2>
    <table>
      <thead>
        <tr>
          <th class="left">Category</th>
          <th class="num">Weight</th>
          <th class="num">Class Avg %</th>
          <th class="num">Students</th>
          <th class="num">Assessments</th>
        </tr>
      </thead>
      <tbody>${perCategoryRows}</tbody>
    </table>

    <div class="break"></div>
    <h2>Assessment Stats</h2>
    <table>
      <thead>
        <tr>
          <th class="left">Assessment</th>
          <th class="left">Category</th>
          <th class="num">Out Of</th>
          <th class="num">Avg Raw</th>
          <th class="num">Avg %</th>
          <th class="num">Median %</th>
          <th class="num">Scored</th>
          <th class="num">Zero</th>
          <th class="num">No Mark</th>
        </tr>
      </thead>
      <tbody>${perAssessmentRows}</tbody>
    </table>
  </body>
</html>`;
}

export type CombinedAnalysisReportModel = {
  class: { id: string; name: string };
  markSets: Array<{
    id: string;
    code: string;
    description: string;
    sortOrder: number;
    weight: number;
  }>;
  filters: {
    term: number | null;
    categoryName: string | null;
    typesMask: number | null;
  };
  studentScope?: "all" | "active" | "valid";
  settingsApplied?: {
    combineMethod: string;
    fallbackUsedCount: number;
  };
  kpis: {
    classAverage: number | null;
    classMedian: number | null;
    studentCount: number;
    finalMarkCount: number;
    noCombinedFinalCount: number;
  };
  distributions: {
    bins: Array<{ label: string; min: number; max: number; count: number }>;
    noCombinedFinalCount: number;
  };
  perMarkSet: Array<{
    markSetId: string;
    code: string;
    description: string;
    weight: number;
    finalMarkCount: number;
    classAverage: number | null;
    classMedian: number | null;
  }>;
  rows: Array<{
    studentId: string;
    displayName: string;
    sortOrder: number;
    active: boolean;
    combinedFinal: number | null;
    perMarkSet: Array<{
      markSetId: string;
      code: string;
      description: string;
      weight: number;
      valid: boolean;
      finalMark: number | null;
    }>;
  }>;
  topBottom: {
    top: Array<{ studentId: string; displayName: string; combinedFinal: number | null }>;
    bottom: Array<{ studentId: string; displayName: string; combinedFinal: number | null }>;
  };
};

export function renderCombinedAnalysisReportHtml(model: CombinedAnalysisReportModel): string {
  const generatedAt = new Date().toLocaleString();
  const markSetHeaders = model.markSets
    .map((ms) => `<th class="num">${escapeHtml(ms.code)}</th>`)
    .join("");
  const studentRows = model.rows
    .map((r) => {
      const perMap = new Map(
        (r.perMarkSet ?? []).map((x) => [x.markSetId, x.finalMark] as const)
      );
      const perCells = model.markSets
        .map((ms) => {
          const v = perMap.get(ms.id);
          return `<td class="num">${v == null ? "" : escapeHtml(v.toFixed(1))}</td>`;
        })
        .join("");
      return `<tr>
        <td class="left">${escapeHtml(r.displayName)}</td>
        <td class="num">${r.combinedFinal == null ? "" : escapeHtml(r.combinedFinal.toFixed(1))}</td>
        ${perCells}
      </tr>`;
    })
    .join("");
  const perMarkSetRows = model.perMarkSet
    .map(
      (ms) => `<tr>
      <td class="left">${escapeHtml(ms.code)}</td>
      <td class="left">${escapeHtml(ms.description)}</td>
      <td class="num">${ms.weight.toFixed(1)}</td>
      <td class="num">${ms.finalMarkCount}</td>
      <td class="num">${ms.classAverage == null ? "" : ms.classAverage.toFixed(1)}</td>
      <td class="num">${ms.classMedian == null ? "" : ms.classMedian.toFixed(1)}</td>
    </tr>`
    )
    .join("");

  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4 landscape; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      h2 { margin: 14px 0 6px 0; font-size: 14px; }
      .meta { font-size: 11px; color: #444; line-height: 1.3; }
      .top { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 8px; }
      .cards { display: grid; grid-template-columns: repeat(5, minmax(110px, 1fr)); gap: 8px; margin: 8px 0 10px; }
      .card { border: 1px solid #ddd; border-radius: 8px; padding: 8px; }
      table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      thead { display: table-header-group; }
      tfoot { display: table-footer-group; }
      tr { break-inside: avoid; page-break-inside: avoid; }
      th, td { border: 1px solid #ccc; font-size: 10px; padding: 3px 4px; }
      th { background: #f6f6f6; }
      td.left, th.left { text-align: left; }
      td.num, th.num { text-align: right; }
      .break { page-break-before: always; }
    </style>
  </head>
  <body>
    <div class="top">
      <div>
        <h1>Combined Analytics</h1>
        <div class="meta">
          <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
          <div><strong>Mark Sets:</strong> ${escapeHtml(
            model.markSets.map((m) => `${m.code}(${m.weight.toFixed(1)})`).join(", ")
          )}</div>
          <div><strong>Scope:</strong> ${escapeHtml(reportStudentScopeLabel(model.studentScope))}</div>
          <div><strong>Filters:</strong> ${escapeHtml(reportFiltersLabel(model.filters))}</div>
        </div>
      </div>
      <div class="meta">${escapeHtml(generatedAt)}</div>
    </div>

    <div class="cards">
      <div class="card"><strong>Class Avg:</strong> ${
        model.kpis.classAverage == null ? "" : model.kpis.classAverage.toFixed(1)
      }</div>
      <div class="card"><strong>Class Median:</strong> ${
        model.kpis.classMedian == null ? "" : model.kpis.classMedian.toFixed(1)
      }</div>
      <div class="card"><strong>Students:</strong> ${model.kpis.studentCount}</div>
      <div class="card"><strong>Final Marks:</strong> ${model.kpis.finalMarkCount}</div>
      <div class="card"><strong>No Combined Final:</strong> ${model.kpis.noCombinedFinalCount}</div>
    </div>

    <h2>Per Mark Set</h2>
    <table>
      <thead>
        <tr>
          <th class="left">Code</th>
          <th class="left">Description</th>
          <th class="num">Weight</th>
          <th class="num">Final Count</th>
          <th class="num">Avg</th>
          <th class="num">Median</th>
        </tr>
      </thead>
      <tbody>${perMarkSetRows}</tbody>
    </table>

    <div class="break"></div>
    <h2>Student Combined Results</h2>
    <table>
      <thead>
        <tr>
          <th class="left">Student</th>
          <th class="num">Combined</th>
          ${markSetHeaders}
        </tr>
      </thead>
      <tbody>${studentRows}</tbody>
    </table>
  </body>
</html>`;
}

export type StudentSummaryReportModel = {
  class: { id: string; name: string };
  markSet: { id: string; code: string; description: string };
  settings: {
    fullCode: string | null;
    room: string | null;
    day: string | null;
    period: string | null;
    weightMethod: number;
    calcMethod: number;
  };
  filters: {
    term: number | null;
    categoryName: string | null;
    typesMask: number | null;
  };
  studentScope?: "all" | "active" | "valid";
  student: {
    studentId: string;
    displayName: string;
    sortOrder: number;
    active: boolean;
    finalMark: number | null;
    noMarkCount: number;
    zeroCount: number;
    scoredCount: number;
  };
  assessments: Array<{
    assessmentId: string;
    idx: number;
    date: string | null;
    categoryName: string | null;
    title: string;
    term: number | null;
    legacyType: number | null;
    weight: number;
    outOf: number;
  }>;
  perAssessment: Array<{
    assessmentId: string;
    idx: number;
    date: string | null;
    categoryName: string | null;
    title: string;
    outOf: number;
    avgRaw: number;
    avgPercent: number;
    medianPercent: number;
    scoredCount: number;
    zeroCount: number;
    noMarkCount: number;
  }>;
};

export type ClassAssessmentDrilldownReportModel = {
  class: { id: string; name: string };
  markSet: { id: string; code: string; description: string };
  filters: {
    term: number | null;
    categoryName: string | null;
    typesMask: number | null;
  };
  studentScope?: "all" | "active" | "valid";
  assessment: {
    assessmentId: string;
    idx: number;
    date: string | null;
    categoryName: string | null;
    title: string;
    term: number | null;
    legacyType: number | null;
    weight: number;
    outOf: number;
  };
  rows: Array<{
    studentId: string;
    displayName: string;
    sortOrder: number;
    active: boolean;
    status: "no_mark" | "zero" | "scored";
    raw: number | null;
    percent: number | null;
    finalMark: number | null;
  }>;
  totalRows: number;
  page: number;
  pageSize: number;
  sortBy: string;
  sortDir: "asc" | "desc";
  classStats: {
    assessmentId: string;
    idx: number;
    date: string | null;
    categoryName: string | null;
    title: string;
    outOf: number;
    avgRaw: number;
    avgPercent: number;
    medianPercent: number;
    scoredCount: number;
    zeroCount: number;
    noMarkCount: number;
  };
};

export function renderClassAssessmentDrilldownReportHtml(
  model: ClassAssessmentDrilldownReportModel
): string {
  const generatedAt = new Date().toLocaleString();
  const rowHtml = model.rows
    .map(
      (r) => `<tr>
      <td class="left">${escapeHtml(r.displayName)}</td>
      <td class="left">${escapeHtml(r.status)}</td>
      <td class="num">${r.raw == null ? "" : escapeHtml(r.raw.toFixed(1))}</td>
      <td class="num">${r.percent == null ? "" : escapeHtml(r.percent.toFixed(1))}</td>
      <td class="num">${r.finalMark == null ? "" : escapeHtml(r.finalMark.toFixed(1))}</td>
    </tr>`
    )
    .join("");

  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4 landscape; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      h2 { margin: 14px 0 6px 0; font-size: 14px; }
      .meta { font-size: 11px; color: #444; line-height: 1.3; }
      .top { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 8px; }
      .grid { display: grid; grid-template-columns: repeat(4, minmax(120px, 1fr)); gap: 8px; margin: 8px 0 10px; }
      .card { border: 1px solid #ddd; border-radius: 8px; padding: 8px; }
      table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      thead { display: table-header-group; }
      tfoot { display: table-footer-group; }
      tr { break-inside: avoid; page-break-inside: avoid; }
      th, td { border: 1px solid #ccc; font-size: 10px; padding: 3px 4px; }
      th { background: #f6f6f6; }
      td.left, th.left { text-align: left; }
      td.num, th.num { text-align: right; }
    </style>
  </head>
  <body>
    <div class="top">
      <div>
        <h1>Class Assessment Drilldown</h1>
        <div class="meta">
          <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
          <div><strong>Mark Set:</strong> ${escapeHtml(model.markSet.code)}: ${escapeHtml(
    model.markSet.description
  )}</div>
          <div><strong>Assessment:</strong> ${escapeHtml(model.assessment.title)}</div>
          <div><strong>Scope:</strong> ${escapeHtml(reportStudentScopeLabel(model.studentScope))}</div>
          <div><strong>Filters:</strong> ${escapeHtml(reportFiltersLabel(model.filters))}</div>
          <div><strong>Query:</strong> ${escapeHtml(
            `sort=${model.sortBy}/${model.sortDir}, page=${model.page}, size=${model.pageSize}`
          )}</div>
        </div>
      </div>
      <div class="meta">${escapeHtml(generatedAt)}</div>
    </div>

    <div class="grid">
      <div class="card"><strong>Avg Raw:</strong> ${model.classStats.avgRaw.toFixed(1)}</div>
      <div class="card"><strong>Avg %:</strong> ${model.classStats.avgPercent.toFixed(1)}</div>
      <div class="card"><strong>Median %:</strong> ${model.classStats.medianPercent.toFixed(1)}</div>
      <div class="card"><strong>Scored/Zero/No Mark:</strong> ${model.classStats.scoredCount}/${model.classStats.zeroCount}/${model.classStats.noMarkCount}</div>
    </div>

    <h2>Student Rows (page ${model.page} of ${Math.max(1, Math.ceil(model.totalRows / Math.max(1, model.pageSize)))})</h2>
    <table>
      <thead>
        <tr>
          <th class="left">Student</th>
          <th class="left">Status</th>
          <th class="num">Raw</th>
          <th class="num">Percent</th>
          <th class="num">Final Mark</th>
        </tr>
      </thead>
      <tbody>${rowHtml}</tbody>
    </table>
  </body>
</html>`;
}

export function renderStudentSummaryReportHtml(model: StudentSummaryReportModel): string {
  const generatedAt = new Date().toLocaleString();
  const statRows = model.perAssessment
    .map(
      (a) => `<tr>
      <td class="left">${escapeHtml(a.title)}</td>
      <td class="left">${escapeHtml(a.categoryName ?? "")}</td>
      <td class="num">${a.outOf.toFixed(1)}</td>
      <td class="num">${a.avgRaw.toFixed(1)}</td>
      <td class="num">${a.avgPercent.toFixed(1)}</td>
      <td class="num">${a.medianPercent.toFixed(1)}</td>
    </tr>`
    )
    .join("");
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      h2 { margin: 14px 0 6px 0; font-size: 14px; }
      .meta { font-size: 11px; color: #444; line-height: 1.3; }
      .top { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 8px; }
      table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      thead { display: table-header-group; }
      tfoot { display: table-footer-group; }
      tr { break-inside: avoid; page-break-inside: avoid; }
      th, td { border: 1px solid #ccc; font-size: 10px; padding: 3px 4px; }
      th { background: #f6f6f6; }
      td.left, th.left { text-align: left; }
      td.num, th.num { text-align: right; }
      .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin-top: 8px; }
      .card { border: 1px solid #ddd; border-radius: 8px; padding: 8px; }
    </style>
  </head>
  <body>
    <div class="top">
      <div>
        <h1>Student Summary</h1>
        <div class="meta">
          <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
          <div><strong>Mark Set:</strong> ${escapeHtml(model.markSet.code)}: ${escapeHtml(
    model.markSet.description
  )}</div>
          <div><strong>Student:</strong> ${escapeHtml(model.student.displayName)}</div>
          <div><strong>Scope:</strong> ${escapeHtml(reportStudentScopeLabel(model.studentScope))}</div>
          <div><strong>Filters:</strong> ${escapeHtml(reportFiltersLabel(model.filters))}</div>
        </div>
      </div>
      <div class="meta">${escapeHtml(generatedAt)}</div>
    </div>

    <div class="grid">
      <div class="card">
        <div><strong>Final Mark:</strong> ${
          model.student.finalMark == null ? "" : model.student.finalMark.toFixed(1)
        }</div>
        <div><strong>Scored:</strong> ${model.student.scoredCount}</div>
      </div>
      <div class="card">
        <div><strong>Zero:</strong> ${model.student.zeroCount}</div>
        <div><strong>No Mark:</strong> ${model.student.noMarkCount}</div>
      </div>
    </div>

    <h2>Assessment Context</h2>
    <table>
      <thead>
        <tr>
          <th class="left">Assessment</th>
          <th class="left">Category</th>
          <th class="num">Out Of</th>
          <th class="num">Class Avg Raw</th>
          <th class="num">Class Avg %</th>
          <th class="num">Class Median %</th>
        </tr>
      </thead>
      <tbody>${statRows}</tbody>
    </table>
  </body>
</html>`;
}

export type AttendanceMonthlyReportModel = {
  class: { id: string; name: string };
  attendance: {
    month: string;
    daysInMonth: number;
    typeOfDayCodes: string;
    students: Array<{ id: string; displayName: string; sortOrder: number; active: boolean }>;
    rows: Array<{ studentId: string; dayCodes: string }>;
  };
};

export function renderAttendanceMonthlyReportHtml(
  model: AttendanceMonthlyReportModel
): string {
  const generatedAt = new Date().toLocaleString();
  const days = Array.from({ length: model.attendance.daysInMonth }, (_, i) => i + 1);
  const rowByStudent = new Map(model.attendance.rows.map((r) => [r.studentId, r.dayCodes]));
  const rows = model.attendance.students
    .map((s) => {
      const dayCodes = rowByStudent.get(s.id) ?? "";
      const cells = days
        .map((d) => `<td class="c">${escapeHtml(dayCodes[d - 1] ?? "")}</td>`)
        .join("");
      return `<tr><td class="left">${escapeHtml(s.displayName)}</td>${cells}</tr>`;
    })
    .join("");
  const typeRow = days
    .map((d) => `<td class="c">${escapeHtml(model.attendance.typeOfDayCodes[d - 1] ?? "")}</td>`)
    .join("");
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4 landscape; margin: 10mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      .meta { font-size: 11px; color: #444; margin-bottom: 8px; }
      table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      thead { display: table-header-group; }
      tfoot { display: table-footer-group; }
      tr { break-inside: avoid; page-break-inside: avoid; }
      th, td { border: 1px solid #ccc; font-size: 9px; padding: 2px 3px; }
      th { background: #f6f6f6; }
      td.left, th.left { text-align: left; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
      td.c, th.c { text-align: center; width: 12px; }
    </style>
  </head>
  <body>
    <h1>Attendance Monthly Report</h1>
    <div class="meta">
      <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
      <div><strong>Month:</strong> ${escapeHtml(model.attendance.month)}</div>
      <div>${escapeHtml(generatedAt)}</div>
    </div>
    <table>
      <thead>
        <tr>
          <th class="left">Student</th>
          ${days.map((d) => `<th class="c">${d}</th>`).join("")}
        </tr>
      </thead>
      <tbody>
        <tr><td class="left"><strong>Type</strong></td>${typeRow}</tr>
        ${rows}
      </tbody>
    </table>
  </body>
</html>`;
}

export type ClassListReportModel = {
  class: { id: string; name: string };
  students: Array<{
    id: string;
    displayName: string;
    studentNo: string | null;
    birthDate: string | null;
    active: boolean;
    sortOrder: number;
    note: string;
  }>;
};

export function renderClassListReportHtml(model: ClassListReportModel): string {
  const generatedAt = new Date().toLocaleString();
  const rows = model.students
    .map(
      (s) => `<tr>
      <td class="left">${escapeHtml(s.displayName)}</td>
      <td>${escapeHtml(s.studentNo ?? "")}</td>
      <td>${escapeHtml(s.birthDate ?? "")}</td>
      <td>${s.active ? "Y" : "N"}</td>
      <td class="left">${escapeHtml(s.note ?? "")}</td>
    </tr>`
    )
    .join("");
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      .meta { font-size: 11px; color: #444; margin-bottom: 8px; }
      table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      thead { display: table-header-group; }
      tfoot { display: table-footer-group; }
      tr { break-inside: avoid; page-break-inside: avoid; }
      th, td { border: 1px solid #ccc; font-size: 10px; padding: 3px 4px; vertical-align: top; }
      th { background: #f6f6f6; }
      td.left, th.left { text-align: left; }
    </style>
  </head>
  <body>
    <h1>Class List</h1>
    <div class="meta">
      <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
      <div>${escapeHtml(generatedAt)}</div>
    </div>
    <table>
      <thead>
        <tr>
          <th class="left">Student</th>
          <th>No</th>
          <th>Birth Date</th>
          <th>Active</th>
          <th class="left">Note</th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>
  </body>
</html>`;
}

export type LearningSkillsSummaryReportModel = {
  class: { id: string; name: string };
  term: number;
  skillCodes: string[];
  students: Array<{ id: string; displayName: string; sortOrder: number; active: boolean }>;
  rows: Array<{ studentId: string; values: Record<string, string> }>;
};

export function renderLearningSkillsSummaryReportHtml(
  model: LearningSkillsSummaryReportModel
): string {
  const generatedAt = new Date().toLocaleString();
  const rowByStudent = new Map(model.rows.map((r) => [r.studentId, r.values]));
  const bodyRows = model.students
    .map((s) => {
      const values = rowByStudent.get(s.id) ?? {};
      const cells = model.skillCodes
        .map((code) => `<td class="c">${escapeHtml(values[code] ?? "")}</td>`)
        .join("");
      return `<tr><td class="left">${escapeHtml(s.displayName)}</td>${cells}</tr>`;
    })
    .join("");
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      .meta { font-size: 11px; color: #444; margin-bottom: 8px; }
      table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      thead { display: table-header-group; }
      tfoot { display: table-footer-group; }
      tr { break-inside: avoid; page-break-inside: avoid; }
      th, td { border: 1px solid #ccc; font-size: 10px; padding: 3px 4px; }
      th { background: #f6f6f6; }
      td.left, th.left { text-align: left; }
      td.c, th.c { text-align: center; width: 20mm; }
    </style>
  </head>
  <body>
    <h1>Learning Skills Summary</h1>
    <div class="meta">
      <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
      <div><strong>Term:</strong> ${model.term}</div>
      <div>${escapeHtml(generatedAt)}</div>
    </div>
    <table>
      <thead>
        <tr>
          <th class="left">Student</th>
          ${model.skillCodes.map((code) => `<th class="c">${escapeHtml(code)}</th>`).join("")}
        </tr>
      </thead>
      <tbody>${bodyRows}</tbody>
    </table>
  </body>
</html>`;
}

export type PlannerUnitReportModel = {
  artifactKind: "unit";
  title: string;
  unit: {
    unitId: string;
    title: string;
    startDate: string | null;
    endDate: string | null;
    summary: string;
    expectations: string[];
    resources: string[];
  };
  lessons: Array<{
    lessonId: string;
    title: string;
    lessonDate: string | null;
    outline: string;
    detail: string;
    followUp: string;
    homework: string;
    durationMinutes: number | null;
  }>;
};

export function renderPlannerUnitReportHtml(model: PlannerUnitReportModel): string {
  const generatedAt = new Date().toLocaleString();
  const lessonRows = model.lessons
    .map(
      (lesson) => `<tr>
        <td>${escapeHtml(lesson.lessonDate ?? "")}</td>
        <td class="left">${escapeHtml(lesson.title)}</td>
        <td>${lesson.durationMinutes == null ? "" : lesson.durationMinutes}</td>
        <td class="left">${escapeHtml(lesson.outline)}</td>
      </tr>`
    )
    .join("");

  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      h2 { margin: 14px 0 6px 0; font-size: 14px; }
      .meta { font-size: 11px; color: #444; line-height: 1.35; margin-bottom: 8px; }
      table { width: 100%; border-collapse: collapse; table-layout: fixed; }
      thead { display: table-header-group; }
      tr { break-inside: avoid; page-break-inside: avoid; }
      th, td { border: 1px solid #ccc; font-size: 10px; padding: 4px; vertical-align: top; }
      th { background: #f6f6f6; }
      .left { text-align: left; }
      .block { border: 1px solid #ddd; border-radius: 8px; padding: 8px; margin: 8px 0; }
      .muted { color: #666; }
    </style>
  </head>
  <body>
    <h1>Planner Unit</h1>
    <div class="meta">
      <div><strong>Unit:</strong> ${escapeHtml(model.unit.title)}</div>
      <div><strong>Date Range:</strong> ${escapeHtml(model.unit.startDate ?? "")} - ${escapeHtml(
    model.unit.endDate ?? ""
  )}</div>
      <div>${escapeHtml(generatedAt)}</div>
    </div>
    <div class="block">
      <strong>Summary</strong>
      <div class="muted">${escapeHtml(model.unit.summary || "(none)")}</div>
    </div>
    <div class="block">
      <strong>Expectations</strong>
      <ul>
        ${(model.unit.expectations.length ? model.unit.expectations : ["(none)"])
          .map((v) => `<li>${escapeHtml(v)}</li>`)
          .join("")}
      </ul>
      <strong>Resources</strong>
      <ul>
        ${(model.unit.resources.length ? model.unit.resources : ["(none)"])
          .map((v) => `<li>${escapeHtml(v)}</li>`)
          .join("")}
      </ul>
    </div>
    <h2>Lessons</h2>
    <table>
      <thead>
        <tr>
          <th>Date</th>
          <th class="left">Title</th>
          <th>Minutes</th>
          <th class="left">Outline</th>
        </tr>
      </thead>
      <tbody>${lessonRows || `<tr><td colspan="4" class="left">(no lessons)</td></tr>`}</tbody>
    </table>
  </body>
</html>`;
}

export type PlannerLessonReportModel = {
  artifactKind: "lesson";
  title: string;
  lesson: {
    lessonId: string;
    unitId: string | null;
    title: string;
    lessonDate: string | null;
    outline: string;
    detail: string;
    followUp: string;
    homework: string;
    durationMinutes: number | null;
    unitTitle: string | null;
  };
};

export function renderPlannerLessonReportHtml(model: PlannerLessonReportModel): string {
  const generatedAt = new Date().toLocaleString();
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      .meta { font-size: 11px; color: #444; line-height: 1.35; margin-bottom: 8px; }
      .block { border: 1px solid #ddd; border-radius: 8px; padding: 8px; margin: 8px 0; white-space: pre-wrap; }
    </style>
  </head>
  <body>
    <h1>Planner Lesson</h1>
    <div class="meta">
      <div><strong>Title:</strong> ${escapeHtml(model.lesson.title)}</div>
      <div><strong>Date:</strong> ${escapeHtml(model.lesson.lessonDate ?? "")}</div>
      <div><strong>Unit:</strong> ${escapeHtml(model.lesson.unitTitle ?? "")}</div>
      <div><strong>Minutes:</strong> ${
        model.lesson.durationMinutes == null ? "" : model.lesson.durationMinutes
      }</div>
      <div>${escapeHtml(generatedAt)}</div>
    </div>
    <div class="block"><strong>Outline</strong><div>${escapeHtml(model.lesson.outline || "(none)")}</div></div>
    <div class="block"><strong>Detail</strong><div>${escapeHtml(model.lesson.detail || "(none)")}</div></div>
    <div class="block"><strong>Follow-up</strong><div>${escapeHtml(model.lesson.followUp || "(none)")}</div></div>
    <div class="block"><strong>Homework</strong><div>${escapeHtml(model.lesson.homework || "(none)")}</div></div>
  </body>
</html>`;
}

export type CourseDescriptionReportModel = {
  class: { id: string; name: string };
  profile: {
    courseTitle: string;
    gradeLabel: string;
    periodMinutes: number;
    periodsPerWeek: number;
    totalWeeks: number;
    strands: Array<string | unknown>;
    policyText: string;
  };
  schedule: {
    periodMinutes: number;
    periodsPerWeek: number;
    totalWeeks: number;
    totalHours: number;
  };
  units: Array<{ title?: string; summary?: string }>;
  lessons: Array<{ title?: string }>;
  generatedAt: string;
};

export function renderCourseDescriptionReportHtml(model: CourseDescriptionReportModel): string {
  const strands = (model.profile.strands ?? []).map((s) => String(s));
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      .meta { font-size: 11px; color: #444; line-height: 1.35; margin-bottom: 8px; }
      .block { border: 1px solid #ddd; border-radius: 8px; padding: 8px; margin: 8px 0; }
      .muted { color: #666; }
    </style>
  </head>
  <body>
    <h1>Course Description</h1>
    <div class="meta">
      <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
      <div><strong>Course:</strong> ${escapeHtml(model.profile.courseTitle)}</div>
      <div><strong>Grade:</strong> ${escapeHtml(model.profile.gradeLabel)}</div>
      <div>${escapeHtml(new Date(Number(model.generatedAt) * 1000 || Date.now()).toLocaleString())}</div>
    </div>
    <div class="block">
      <strong>Schedule</strong>
      <div class="muted">${model.schedule.periodMinutes} min x ${model.schedule.periodsPerWeek} / week x ${
    model.schedule.totalWeeks
  } weeks (${model.schedule.totalHours.toFixed(1)} hours)</div>
    </div>
    <div class="block">
      <strong>Strands</strong>
      <ul>${(strands.length ? strands : ["(none)"]).map((s) => `<li>${escapeHtml(s)}</li>`).join("")}</ul>
    </div>
    <div class="block">
      <strong>Policy</strong>
      <div class="muted">${escapeHtml(model.profile.policyText || "(none)")}</div>
    </div>
    <div class="block">
      <strong>Planned Units</strong>
      <div class="muted">${model.units.length} units, ${model.lessons.length} lessons</div>
    </div>
  </body>
</html>`;
}

export type TimeManagementReportModel = {
  class: { id: string; name: string };
  inputs: {
    periodMinutes: number;
    periodsPerWeek: number;
    totalWeeks: number;
    includeArchived: boolean;
  };
  totals: {
    plannedMinutes: number;
    availableMinutes: number;
    remainingMinutes: number;
    utilizationPercent: number;
  };
  generatedAt: string;
};

export function renderTimeManagementReportHtml(model: TimeManagementReportModel): string {
  const generatedAt = new Date(Number(model.generatedAt) * 1000 || Date.now()).toLocaleString();
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <style>
      @page { size: A4; margin: 12mm; }
      body { font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", Helvetica, Arial, sans-serif; color: #111; }
      h1 { margin: 0; font-size: 18px; }
      .meta { font-size: 11px; color: #444; line-height: 1.35; margin-bottom: 8px; }
      table { width: 100%; border-collapse: collapse; margin-top: 8px; }
      th, td { border: 1px solid #ccc; font-size: 10px; padding: 4px; }
      th { background: #f6f6f6; text-align: left; width: 40%; }
    </style>
  </head>
  <body>
    <h1>Time Management</h1>
    <div class="meta">
      <div><strong>Class:</strong> ${escapeHtml(model.class.name)}</div>
      <div>${escapeHtml(generatedAt)}</div>
    </div>
    <table>
      <tbody>
        <tr><th>Period Minutes</th><td>${model.inputs.periodMinutes}</td></tr>
        <tr><th>Periods Per Week</th><td>${model.inputs.periodsPerWeek}</td></tr>
        <tr><th>Total Weeks</th><td>${model.inputs.totalWeeks}</td></tr>
        <tr><th>Include Archived</th><td>${model.inputs.includeArchived ? "Yes" : "No"}</td></tr>
        <tr><th>Planned Minutes</th><td>${model.totals.plannedMinutes}</td></tr>
        <tr><th>Available Minutes</th><td>${model.totals.availableMinutes}</td></tr>
        <tr><th>Remaining Minutes</th><td>${model.totals.remainingMinutes}</td></tr>
        <tr><th>Utilization</th><td>${model.totals.utilizationPercent.toFixed(1)}%</td></tr>
      </tbody>
    </table>
  </body>
</html>`;
}

function escapeHtml(s: string) {
  return s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}
