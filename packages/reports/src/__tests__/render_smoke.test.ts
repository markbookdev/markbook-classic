import { describe, expect, test } from "bun:test";
import {
  renderMarkSetGridReportHtml,
  renderMarkSetSummaryReportHtml,
  renderCategoryAnalysisReportHtml,
  renderStudentSummaryReportHtml,
  renderAttendanceMonthlyReportHtml,
  renderClassListReportHtml,
  renderLearningSkillsSummaryReportHtml
} from "../index";

function expectTablePrintCss(html: string) {
  expect(html).toContain("table-header-group");
  expect(html).toContain("break-inside");
}

describe("reports html render smoke", () => {
  test("mark set grid includes print css", () => {
    const html = renderMarkSetGridReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      markSet: { id: "ms1", code: "MAT1", description: "Mathematics 1" },
      students: [{ id: "s1", displayName: "Boyce, Daniella", sortOrder: 0, active: true }],
      assessments: [
        {
          id: "a1",
          idx: 0,
          date: "2025-09-01",
          categoryName: "KNOW",
          title: "Review",
          weight: 1,
          outOf: 10
        }
      ],
      assessmentAverages: [
        {
          assessmentId: "a1",
          idx: 0,
          avgRaw: 5,
          avgPercent: 50,
          scoredCount: 1,
          zeroCount: 0,
          noMarkCount: 0
        }
      ],
      cells: [[5]]
    });
    expectTablePrintCss(html);
    expect(html).toContain("Mark Set Grid");
  });

  test("mark set summary includes print css", () => {
    const html = renderMarkSetSummaryReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      markSet: { id: "ms1", code: "MAT1", description: "Mathematics 1" },
      settings: {
        fullCode: null,
        room: null,
        day: null,
        period: null,
        weightMethod: 1,
        calcMethod: 0
      },
      filters: { term: null, categoryName: null, typesMask: null },
      categories: [{ name: "KNOW", weight: 100, sortOrder: 0 }],
      assessments: [],
      perAssessment: [],
      perCategory: [
        { name: "KNOW", weight: 100, sortOrder: 0, classAvg: 50, studentCount: 1, assessmentCount: 1 }
      ],
      perStudent: [
        {
          studentId: "s1",
          displayName: "Boyce, Daniella",
          sortOrder: 0,
          active: true,
          finalMark: 50,
          noMarkCount: 0,
          zeroCount: 0,
          scoredCount: 1
        }
      ]
    });
    expectTablePrintCss(html);
    expect(html).toContain("Mark Set Summary");
  });

  test("category analysis includes print css", () => {
    const html = renderCategoryAnalysisReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      markSet: { id: "ms1", code: "MAT1", description: "Mathematics 1" },
      settings: { fullCode: null, room: null, day: null, period: null, weightMethod: 1, calcMethod: 0 },
      filters: { term: null, categoryName: null, typesMask: null },
      categories: [{ name: "KNOW", weight: 100, sortOrder: 0 }],
      perCategory: [
        { name: "KNOW", weight: 100, sortOrder: 0, classAvg: 50, studentCount: 1, assessmentCount: 1 }
      ],
      perAssessment: []
    });
    expectTablePrintCss(html);
    expect(html).toContain("Category Analysis");
  });

  test("student summary includes print css", () => {
    const html = renderStudentSummaryReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      markSet: { id: "ms1", code: "MAT1", description: "Mathematics 1" },
      settings: { fullCode: null, room: null, day: null, period: null, weightMethod: 1, calcMethod: 0 },
      filters: { term: null, categoryName: null, typesMask: null },
      student: {
        studentId: "s1",
        displayName: "Boyce, Daniella",
        sortOrder: 0,
        active: true,
        finalMark: 50,
        noMarkCount: 0,
        zeroCount: 0,
        scoredCount: 1
      },
      assessments: [],
      perAssessment: []
    });
    expectTablePrintCss(html);
    expect(html).toContain("Student Summary");
  });

  test("attendance includes print css", () => {
    const html = renderAttendanceMonthlyReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      attendance: {
        month: "2025-09",
        daysInMonth: 30,
        typeOfDayCodes: "..............................",
        students: [{ id: "s1", displayName: "Boyce, Daniella", sortOrder: 0, active: true }],
        rows: [{ studentId: "s1", dayCodes: ".............................." }]
      }
    });
    expectTablePrintCss(html);
    expect(html).toContain("Attendance Monthly Report");
  });

  test("class list includes print css", () => {
    const html = renderClassListReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      students: [
        {
          id: "s1",
          displayName: "Boyce, Daniella",
          studentNo: "123",
          birthDate: null,
          active: true,
          sortOrder: 0,
          note: ""
        }
      ]
    });
    expectTablePrintCss(html);
    expect(html).toContain("Class List");
  });

  test("learning skills includes print css", () => {
    const html = renderLearningSkillsSummaryReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      term: 1,
      skillCodes: ["LS1"],
      students: [{ id: "s1", displayName: "Boyce, Daniella", sortOrder: 0, active: true }],
      rows: [{ studentId: "s1", values: { LS1: "G" } }]
    });
    expectTablePrintCss(html);
    expect(html).toContain("Learning Skills Summary");
  });
});

