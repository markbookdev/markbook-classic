import { z } from "zod";

export const SidecarOkResponseSchema = z.object({
  id: z.string(),
  ok: z.literal(true),
  result: z.unknown()
});

export const SidecarErrResponseSchema = z.object({
  id: z.string(),
  ok: z.literal(false),
  error: z.object({
    code: z.string(),
    message: z.string(),
    details: z.record(z.string(), z.unknown()).optional()
  })
});

export const SidecarResponseSchema = z.union([
  SidecarOkResponseSchema,
  SidecarErrResponseSchema
]);

export type SidecarResponse = z.infer<typeof SidecarResponseSchema>;

// Sidecar `result` payload schemas (renderer validates these).

export const ClassesListResultSchema = z.object({
  classes: z.array(
    z.object({
      id: z.string(),
      name: z.string(),
      // Optional to preserve compatibility as the schema evolves.
      studentCount: z.number().optional(),
      markSetCount: z.number().optional()
    })
  )
});

export const ClassesCreateResultSchema = z.object({
  classId: z.string(),
  name: z.string()
});

export const ClassesDeleteResultSchema = z.object({
  ok: z.literal(true)
});

export const ClassImportLegacyResultSchema = z.object({
  classId: z.string(),
  name: z.string(),
  studentsImported: z.number(),
  markSetsImported: z.number().optional(),
  assessmentsImported: z.number().optional(),
  scoresImported: z.number().optional(),
  sourceClFile: z.string(),
  importedMarkFiles: z.array(z.string()).optional(),
  missingMarkFiles: z.array(z.unknown()).optional(),
  loanedItemsImported: z.number().optional(),
  deviceMappingsImported: z.number().optional(),
  combinedCommentSetsImported: z.number().optional()
});

export const MarkSetsListResultSchema = z.object({
  markSets: z.array(
    z.object({
      id: z.string(),
      code: z.string(),
      description: z.string(),
      sortOrder: z.number()
    })
  )
});

export const MarkSetOpenResultSchema = z.object({
  markSet: z.object({
    id: z.string(),
    code: z.string(),
    description: z.string()
  }),
  students: z.array(
    z.object({
      id: z.string(),
      displayName: z.string(),
      sortOrder: z.number(),
      active: z.boolean()
    })
  ),
  assessments: z.array(
    z.object({
      id: z.string(),
      idx: z.number(),
      date: z.string().nullable(),
      categoryName: z.string().nullable(),
      title: z.string(),
      weight: z.number().nullable(),
      outOf: z.number().nullable()
    })
  ),
  rowCount: z.number(),
  colCount: z.number()
});

export const GridGetResultSchema = z.object({
  rowStart: z.number(),
  rowCount: z.number(),
  colStart: z.number(),
  colCount: z.number(),
  cells: z.array(z.array(z.number().nullable()))
});

export const GridUpdateCellResultSchema = z.object({
  ok: z.literal(true)
});

export const GridSetStateResultSchema = z.object({
  ok: z.literal(true)
});

export const GridBulkUpdateResultSchema = z.object({
  ok: z.literal(true),
  updated: z.number()
});

export const ReportsMarkSetGridModelResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  markSet: z.object({
    id: z.string(),
    code: z.string(),
    description: z.string()
  }),
  students: z.array(
    z.object({
      id: z.string(),
      displayName: z.string(),
      sortOrder: z.number(),
      active: z.boolean()
    })
  ),
  assessments: z.array(
    z.object({
      id: z.string(),
      idx: z.number(),
      date: z.string().nullable(),
      categoryName: z.string().nullable(),
      title: z.string(),
      weight: z.number().nullable(),
      outOf: z.number().nullable()
    })
  ),
  rowCount: z.number(),
  colCount: z.number(),
  assessmentAverages: z.array(
    z.object({
      assessmentId: z.string(),
      idx: z.number(),
      avgRaw: z.number(),
      avgPercent: z.number(),
      scoredCount: z.number(),
      zeroCount: z.number(),
      noMarkCount: z.number()
    })
  ),
  cells: z.array(z.array(z.number().nullable()))
});

export const StudentsListResultSchema = z.object({
  students: z.array(
    z.object({
      id: z.string(),
      lastName: z.string(),
      firstName: z.string(),
      displayName: z.string(),
      studentNo: z.string().nullable(),
      birthDate: z.string().nullable(),
      active: z.boolean(),
      sortOrder: z.number()
    })
  )
});

export const StudentsCreateResultSchema = z.object({
  studentId: z.string()
});

export const StudentsUpdateResultSchema = z.object({
  ok: z.literal(true)
});

export const StudentsReorderResultSchema = z.object({
  ok: z.literal(true)
});

export const StudentsDeleteResultSchema = z.object({
  ok: z.literal(true)
});

export const StudentsMembershipGetResultSchema = z.object({
  markSets: z.array(
    z.object({
      id: z.string(),
      code: z.string(),
      sortOrder: z.number()
    })
  ),
  students: z.array(
    z.object({
      id: z.string(),
      displayName: z.string(),
      active: z.boolean(),
      sortOrder: z.number(),
      mask: z.string()
    })
  )
});

export const StudentsMembershipSetResultSchema = z.object({
  ok: z.literal(true),
  mask: z.string()
});

export const CategoriesListResultSchema = z.object({
  categories: z.array(
    z.object({
      id: z.string(),
      name: z.string(),
      weight: z.number().nullable(),
      sortOrder: z.number()
    })
  )
});

export const CategoriesCreateResultSchema = z.object({
  categoryId: z.string()
});

export const CategoriesUpdateResultSchema = z.object({
  ok: z.literal(true)
});

export const CategoriesDeleteResultSchema = z.object({
  ok: z.literal(true)
});

export const AssessmentsListResultSchema = z.object({
  assessments: z.array(
    z.object({
      id: z.string(),
      idx: z.number(),
      date: z.string().nullable(),
      categoryName: z.string().nullable(),
      title: z.string(),
      term: z.number().nullable(),
      legacyType: z.number().nullable(),
      weight: z.number().nullable(),
      outOf: z.number().nullable()
    })
  )
});

export const AssessmentsCreateResultSchema = z.object({
  assessmentId: z.string()
});

export const AssessmentsUpdateResultSchema = z.object({
  ok: z.literal(true)
});

export const AssessmentsDeleteResultSchema = z.object({
  ok: z.literal(true)
});

export const AssessmentsReorderResultSchema = z.object({
  ok: z.literal(true)
});

export const NotesGetResultSchema = z.object({
  notes: z.array(
    z.object({
      studentId: z.string(),
      note: z.string()
    })
  )
});

export const NotesUpdateResultSchema = z.object({
  ok: z.literal(true)
});

export const LoanedListResultSchema = z.object({
  items: z.array(
    z.object({
      id: z.string(),
      studentId: z.string(),
      displayName: z.string(),
      markSetId: z.string().nullable(),
      itemName: z.string(),
      quantity: z.number().nullable(),
      notes: z.string().nullable(),
      rawLine: z.string()
    })
  )
});

export const LoanedGetResultSchema = z.object({
  item: z.object({
    id: z.string(),
    studentId: z.string(),
    displayName: z.string(),
    markSetId: z.string().nullable(),
    itemName: z.string(),
    quantity: z.number().nullable(),
    notes: z.string().nullable(),
    rawLine: z.string()
  })
});

export const LoanedUpdateResultSchema = z.object({
  ok: z.literal(true),
  itemId: z.string()
});

export const DevicesListResultSchema = z.object({
  devices: z.array(
    z.object({
      studentId: z.string(),
      displayName: z.string(),
      sortOrder: z.number(),
      active: z.boolean(),
      deviceCode: z.string(),
      rawLine: z.string()
    })
  )
});

export const DevicesGetResultSchema = z.object({
  device: z.object({
    studentId: z.string(),
    displayName: z.string(),
    sortOrder: z.number(),
    active: z.boolean(),
    deviceCode: z.string(),
    rawLine: z.string()
  })
});

export const DevicesUpdateResultSchema = z.object({
  ok: z.literal(true)
});

export const MarkSetSettingsGetResultSchema = z.object({
  markSet: z.object({
    id: z.string(),
    code: z.string(),
    description: z.string(),
    fullCode: z.string().nullable(),
    room: z.string().nullable(),
    day: z.string().nullable(),
    period: z.string().nullable(),
    weightMethod: z.number(),
    calcMethod: z.number()
  })
});

export const MarkSetSettingsUpdateResultSchema = z.object({
  ok: z.literal(true)
});

const CalcPerAssessmentSchema = z.object({
  assessmentId: z.string(),
  idx: z.number(),
  date: z.string().nullable(),
  categoryName: z.string().nullable(),
  title: z.string(),
  outOf: z.number(),
  avgRaw: z.number(),
  avgPercent: z.number(),
  medianPercent: z.number(),
  scoredCount: z.number(),
  zeroCount: z.number(),
  noMarkCount: z.number()
});

const CalcPerCategorySchema = z.object({
  name: z.string(),
  weight: z.number(),
  sortOrder: z.number().nullable(),
  classAvg: z.number(),
  studentCount: z.number(),
  assessmentCount: z.number()
});

const CalcPerStudentSchema = z.object({
  studentId: z.string(),
  displayName: z.string(),
  sortOrder: z.number(),
  active: z.boolean(),
  finalMark: z.number().nullable(),
  noMarkCount: z.number(),
  zeroCount: z.number(),
  scoredCount: z.number()
});

const CalcSettingsAppliedSchema = z.object({
  weightMethodApplied: z.number(),
  calcMethodApplied: z.number(),
  roffApplied: z.boolean(),
  modeActiveLevels: z.number(),
  modeLevelVals: z.array(z.number())
});

const CalcPerStudentCategoryBreakdownSchema = z.object({
  studentId: z.string(),
  categories: z.array(
    z.object({
      name: z.string(),
      value: z.number().nullable(),
      weight: z.number(),
      hasData: z.boolean()
    })
  )
});

export const CalcAssessmentStatsResultSchema = z.object({
  assessments: z.array(CalcPerAssessmentSchema)
});

export const CalcMarkSetSummaryResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  markSet: z.object({
    id: z.string(),
    code: z.string(),
    description: z.string()
  }),
  settings: z.object({
    fullCode: z.string().nullable(),
    room: z.string().nullable(),
    day: z.string().nullable(),
    period: z.string().nullable(),
    weightMethod: z.number(),
    calcMethod: z.number()
  }),
  filters: z.object({
    term: z.number().nullable(),
    categoryName: z.string().nullable(),
    typesMask: z.number().nullable()
  }),
  categories: z.array(
    z.object({
      name: z.string(),
      weight: z.number(),
      sortOrder: z.number()
    })
  ),
  assessments: z.array(
    z.object({
      assessmentId: z.string(),
      idx: z.number(),
      date: z.string().nullable(),
      categoryName: z.string().nullable(),
      title: z.string(),
      term: z.number().nullable(),
      legacyType: z.number().nullable(),
      weight: z.number(),
      outOf: z.number()
    })
  ),
  perAssessment: z.array(CalcPerAssessmentSchema),
  perCategory: z.array(CalcPerCategorySchema),
  perStudent: z.array(CalcPerStudentSchema),
  settingsApplied: CalcSettingsAppliedSchema.optional(),
  perStudentCategories: z.array(CalcPerStudentCategoryBreakdownSchema).optional(),
  parityDiagnostics: z
    .object({
      calcMethodApplied: z.number(),
      weightMethodApplied: z.number(),
      selectedAssessmentCount: z.number(),
      selectedCategoryCount: z.number()
    })
    .optional()
});

export const ReportsMarkSetSummaryModelResultSchema = CalcMarkSetSummaryResultSchema;

export const CalcConfigGetResultSchema = z.object({
  source: z.object({
    basePresent: z.boolean(),
    overridePresent: z.boolean()
  }),
  roff: z.boolean(),
  modeActiveLevels: z.number(),
  modeVals: z.array(z.number()).length(22),
  modeSymbols: z.array(z.string()).length(22)
});

export const CalcConfigUpdateResultSchema = z.object({
  ok: z.literal(true)
});

export const CalcConfigClearOverrideResultSchema = z.object({
  ok: z.literal(true)
});

export const AttendanceMonthOpenResultSchema = z.object({
  schoolYearStartMonth: z.number(),
  month: z.string(),
  daysInMonth: z.number(),
  typeOfDayCodes: z.string(),
  students: z.array(
    z.object({
      id: z.string(),
      displayName: z.string(),
      sortOrder: z.number(),
      active: z.boolean()
    })
  ),
  rows: z.array(
    z.object({
      studentId: z.string(),
      dayCodes: z.string()
    })
  )
});

export const AttendanceSetTypeOfDayResultSchema = z.object({
  ok: z.literal(true)
});

export const AttendanceSetStudentDayResultSchema = z.object({
  ok: z.literal(true)
});

export const AttendanceBulkStampDayResultSchema = z.object({
  ok: z.literal(true)
});

export const SeatingGetResultSchema = z.object({
  rows: z.number(),
  seatsPerRow: z.number(),
  blockedSeatCodes: z.array(z.number()),
  assignments: z.array(z.number().nullable())
});

export const SeatingSaveResultSchema = z.object({
  ok: z.literal(true)
});

export const CommentsSetsListResultSchema = z.object({
  sets: z.array(
    z.object({
      setNumber: z.number(),
      title: z.string(),
      fitMode: z.number(),
      fitFontSize: z.number(),
      fitWidth: z.number(),
      fitLines: z.number(),
      fitSubj: z.string(),
      maxChars: z.number(),
      isDefault: z.boolean(),
      bankShort: z.string().nullable()
    })
  )
});

export const CommentsSetsOpenResultSchema = z.object({
  set: z.object({
    id: z.string(),
    setNumber: z.number(),
    title: z.string(),
    fitMode: z.number(),
    fitFontSize: z.number(),
    fitWidth: z.number(),
    fitLines: z.number(),
    fitSubj: z.string(),
    maxChars: z.number(),
    isDefault: z.boolean(),
    bankShort: z.string().nullable()
  }),
  remarksByStudent: z.array(
    z.object({
      studentId: z.string(),
      displayName: z.string(),
      sortOrder: z.number(),
      active: z.boolean(),
      remark: z.string()
    })
  )
});

export const CommentsSetsUpsertResultSchema = z.object({
  setNumber: z.number()
});

export const CommentsSetsDeleteResultSchema = z.object({
  ok: z.literal(true)
});

export const CommentsBanksListResultSchema = z.object({
  banks: z.array(
    z.object({
      id: z.string(),
      shortName: z.string(),
      isDefault: z.boolean(),
      fitProfile: z.string().nullable(),
      sourcePath: z.string().nullable(),
      entryCount: z.number()
    })
  )
});

export const CommentsBanksOpenResultSchema = z.object({
  bank: z.object({
    id: z.string(),
    shortName: z.string(),
    isDefault: z.boolean(),
    fitProfile: z.string().nullable(),
    sourcePath: z.string().nullable()
  }),
  entries: z.array(
    z.object({
      id: z.string(),
      sortOrder: z.number(),
      typeCode: z.string(),
      levelCode: z.string(),
      text: z.string()
    })
  )
});

export const CommentsBanksCreateResultSchema = z.object({
  bankId: z.string()
});

export const CommentsBanksUpdateMetaResultSchema = z.object({
  ok: z.literal(true)
});

export const CommentsBanksEntryUpsertResultSchema = z.object({
  entryId: z.string()
});

export const CommentsBanksEntryDeleteResultSchema = z.object({
  ok: z.literal(true)
});

export const CommentsBanksImportBnkResultSchema = z.object({
  bankId: z.string()
});

export const CommentsBanksExportBnkResultSchema = z.object({
  ok: z.literal(true)
});

export const ReportsCategoryAnalysisModelResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  markSet: z.object({
    id: z.string(),
    code: z.string(),
    description: z.string()
  }),
  settings: z.object({
    fullCode: z.string().nullable(),
    room: z.string().nullable(),
    day: z.string().nullable(),
    period: z.string().nullable(),
    weightMethod: z.number(),
    calcMethod: z.number()
  }),
  filters: z.object({
    term: z.number().nullable(),
    categoryName: z.string().nullable(),
    typesMask: z.number().nullable()
  }),
  categories: z.array(
    z.object({
      name: z.string(),
      weight: z.number(),
      sortOrder: z.number()
    })
  ),
  perCategory: z.array(CalcPerCategorySchema),
  perAssessment: z.array(CalcPerAssessmentSchema)
});

export const ReportsStudentSummaryModelResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  markSet: z.object({
    id: z.string(),
    code: z.string(),
    description: z.string()
  }),
  settings: z.object({
    fullCode: z.string().nullable(),
    room: z.string().nullable(),
    day: z.string().nullable(),
    period: z.string().nullable(),
    weightMethod: z.number(),
    calcMethod: z.number()
  }),
  filters: z.object({
    term: z.number().nullable(),
    categoryName: z.string().nullable(),
    typesMask: z.number().nullable()
  }),
  student: CalcPerStudentSchema,
  assessments: z.array(
    z.object({
      assessmentId: z.string(),
      idx: z.number(),
      date: z.string().nullable(),
      categoryName: z.string().nullable(),
      title: z.string(),
      term: z.number().nullable(),
      legacyType: z.number().nullable(),
      weight: z.number(),
      outOf: z.number()
    })
  ),
  perAssessment: z.array(CalcPerAssessmentSchema)
});

export const LearningSkillsOpenResultSchema = z.object({
  classId: z.string(),
  term: z.number(),
  skillCodes: z.array(z.string()),
  students: z.array(
    z.object({
      id: z.string(),
      displayName: z.string(),
      sortOrder: z.number(),
      active: z.boolean()
    })
  ),
  rows: z.array(
    z.object({
      studentId: z.string(),
      values: z.record(z.string(), z.string())
    })
  )
});

export const LearningSkillsUpdateCellResultSchema = z.object({
  ok: z.literal(true)
});

export const LearningSkillsReportModelResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  classId: z.string(),
  term: z.number(),
  skillCodes: z.array(z.string()),
  students: z.array(
    z.object({
      id: z.string(),
      displayName: z.string(),
      sortOrder: z.number(),
      active: z.boolean()
    })
  ),
  rows: z.array(
    z.object({
      studentId: z.string(),
      values: z.record(z.string(), z.string())
    })
  )
});

export const BackupExportWorkspaceBundleResultSchema = z.object({
  ok: z.literal(true),
  path: z.string(),
  manifestPath: z.string().optional(),
  bundleFormat: z.string().optional(),
  entryCount: z.number().optional()
});

export const BackupImportWorkspaceBundleResultSchema = z.object({
  ok: z.literal(true),
  workspacePath: z.string().optional(),
  bundleFormatDetected: z.string().optional()
});

export const ExchangeExportClassCsvResultSchema = z.object({
  ok: z.literal(true),
  rowsExported: z.number(),
  path: z.string()
});

export const ExchangeImportClassCsvResultSchema = z.object({
  ok: z.literal(true),
  updated: z.number()
});

export const ReportsAttendanceMonthlyModelResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  attendance: AttendanceMonthOpenResultSchema
});

export const ReportsClassListModelResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  students: z.array(
    z.object({
      id: z.string(),
      displayName: z.string(),
      studentNo: z.string().nullable(),
      birthDate: z.string().nullable(),
      active: z.boolean(),
      sortOrder: z.number(),
      note: z.string()
    })
  )
});

export const ReportsLearningSkillsSummaryModelResultSchema =
  LearningSkillsReportModelResultSchema;
