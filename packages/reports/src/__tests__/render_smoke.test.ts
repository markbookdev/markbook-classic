import { describe, expect, test } from "bun:test";
import {
  renderMarkSetGridReportHtml,
  renderMarkSetSummaryReportHtml,
  renderCategoryAnalysisReportHtml,
  renderCombinedAnalysisReportHtml,
  renderClassAssessmentDrilldownReportHtml,
  renderStudentSummaryReportHtml,
  renderAttendanceMonthlyReportHtml,
  renderClassListReportHtml,
  renderLearningSkillsSummaryReportHtml,
  renderPlannerUnitReportHtml,
  renderPlannerLessonReportHtml,
  renderCourseDescriptionReportHtml,
  renderTimeManagementReportHtml
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

  test("class assessment drilldown includes print css", () => {
    const html = renderClassAssessmentDrilldownReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      markSet: { id: "ms1", code: "MAT1", description: "Mathematics 1" },
      filters: { term: 1, categoryName: null, typesMask: null },
      studentScope: "valid",
      assessment: {
        assessmentId: "a1",
        idx: 0,
        date: "2025-09-01",
        categoryName: "KNOW",
        title: "Review Quiz",
        term: 1,
        legacyType: 0,
        weight: 1,
        outOf: 10
      },
      rows: [
        {
          studentId: "s1",
          displayName: "Boyce, Daniella",
          sortOrder: 0,
          active: true,
          status: "scored",
          raw: 8,
          percent: 80,
          finalMark: 78
        }
      ],
      totalRows: 1,
      page: 1,
      pageSize: 25,
      sortBy: "sortOrder",
      sortDir: "asc",
      classStats: {
        assessmentId: "a1",
        idx: 0,
        date: "2025-09-01",
        categoryName: "KNOW",
        title: "Review Quiz",
        outOf: 10,
        avgRaw: 8,
        avgPercent: 80,
        medianPercent: 80,
        scoredCount: 1,
        zeroCount: 0,
        noMarkCount: 0
      }
    });
    expectTablePrintCss(html);
    expect(html).toContain("Class Assessment Drilldown");
  });

  test("combined analysis includes print css", () => {
    const html = renderCombinedAnalysisReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      markSets: [
        { id: "ms1", code: "MAT1", description: "Math 1", sortOrder: 0, weight: 50 },
        { id: "ms2", code: "SNC1", description: "Science 1", sortOrder: 1, weight: 50 }
      ],
      filters: { term: null, categoryName: null, typesMask: null },
      studentScope: "valid",
      settingsApplied: { combineMethod: "weighted_markset", fallbackUsedCount: 0 },
      kpis: {
        classAverage: 75,
        classMedian: 75,
        studentCount: 1,
        finalMarkCount: 1,
        noCombinedFinalCount: 0
      },
      distributions: {
        bins: [{ label: "70-79", min: 70, max: 79.9, count: 1 }],
        noCombinedFinalCount: 0
      },
      perMarkSet: [
        {
          markSetId: "ms1",
          code: "MAT1",
          description: "Math 1",
          weight: 50,
          finalMarkCount: 1,
          classAverage: 80,
          classMedian: 80
        }
      ],
      rows: [
        {
          studentId: "s1",
          displayName: "Boyce, Daniella",
          sortOrder: 0,
          active: true,
          combinedFinal: 75,
          perMarkSet: [
            {
              markSetId: "ms1",
              code: "MAT1",
              description: "Math 1",
              weight: 50,
              valid: true,
              finalMark: 80
            }
          ]
        }
      ],
      topBottom: {
        top: [{ studentId: "s1", displayName: "Boyce, Daniella", combinedFinal: 75 }],
        bottom: [{ studentId: "s1", displayName: "Boyce, Daniella", combinedFinal: 75 }]
      }
    });
    expectTablePrintCss(html);
    expect(html).toContain("Combined Analytics");
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

  test("planner unit includes print css", () => {
    const html = renderPlannerUnitReportHtml({
      artifactKind: "unit",
      title: "Unit 1",
      unit: {
        unitId: "u1",
        title: "Unit 1",
        startDate: "2025-09-01",
        endDate: "2025-09-20",
        summary: "Intro unit",
        expectations: ["Expect 1"],
        resources: ["Resource 1"]
      },
      lessons: [
        {
          lessonId: "l1",
          title: "Lesson 1",
          lessonDate: "2025-09-02",
          outline: "Outline",
          detail: "Detail",
          followUp: "Follow",
          homework: "HW",
          durationMinutes: 75
        }
      ]
    });
    expectTablePrintCss(html);
    expect(html).toContain("Planner Unit");
  });

  test("planner lesson renders", () => {
    const html = renderPlannerLessonReportHtml({
      artifactKind: "lesson",
      title: "Lesson 1",
      lesson: {
        lessonId: "l1",
        unitId: "u1",
        title: "Lesson 1",
        lessonDate: "2025-09-02",
        outline: "Outline",
        detail: "Detail",
        followUp: "Follow up",
        homework: "Homework",
        durationMinutes: 75,
        unitTitle: "Unit 1"
      }
    });
    expect(html).toContain("Planner Lesson");
    expect(html).toContain("Outline");
  });

  test("course description renders", () => {
    const html = renderCourseDescriptionReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      profile: {
        courseTitle: "Course A",
        gradeLabel: "Grade 9",
        periodMinutes: 75,
        periodsPerWeek: 5,
        totalWeeks: 36,
        strands: ["S1"],
        policyText: "Policy"
      },
      schedule: { periodMinutes: 75, periodsPerWeek: 5, totalWeeks: 36, totalHours: 225 },
      units: [{ title: "Unit 1", summary: "Summary" }],
      lessons: [{ title: "Lesson 1" }],
      generatedAt: "0"
    });
    expect(html).toContain("Course Description");
    expect(html).toContain("Course A");
  });

  test("time management renders", () => {
    const html = renderTimeManagementReportHtml({
      class: { id: "c1", name: "8D (2025)" },
      inputs: {
        periodMinutes: 75,
        periodsPerWeek: 5,
        totalWeeks: 36,
        includeArchived: false
      },
      totals: {
        plannedMinutes: 1500,
        availableMinutes: 13500,
        remainingMinutes: 12000,
        utilizationPercent: 11.1
      },
      generatedAt: "0"
    });
    expect(html).toContain("Time Management");
    expect(html).toContain("Utilization");
  });
});
