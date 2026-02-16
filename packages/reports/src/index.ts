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

function escapeHtml(s: string) {
  return s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}
