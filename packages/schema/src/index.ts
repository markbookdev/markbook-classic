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

export const ClassesWizardDefaultsResultSchema = z.object({
  defaults: z.object({
    name: z.string(),
    classCode: z.string(),
    schoolYear: z.string(),
    schoolName: z.string(),
    teacherName: z.string(),
    calcMethodDefault: z.number(),
    weightMethodDefault: z.number(),
    schoolYearStartMonth: z.number()
  })
});

export const ClassesCreateFromWizardResultSchema = z.object({
  classId: z.string(),
  name: z.string(),
  classCode: z.string()
});

export const ClassesMetaGetResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  meta: z.object({
    classCode: z.string().nullable(),
    schoolYear: z.string().nullable(),
    schoolName: z.string().nullable(),
    teacherName: z.string().nullable(),
    calcMethodDefault: z.number().nullable(),
    weightMethodDefault: z.number().nullable(),
    schoolYearStartMonth: z.number().nullable(),
    createdFromWizard: z.boolean(),
    legacyFolderPath: z.string().nullable().optional(),
    legacyClFile: z.string().nullable().optional(),
    legacyYearToken: z.string().nullable().optional(),
    lastImportedAt: z.string().nullable().optional(),
    lastImportWarningsCount: z.number().optional()
  })
});

export const ClassesMetaUpdateResultSchema = z.object({
  ok: z.literal(true)
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

export const ClassesLegacyPreviewResultSchema = z.object({
  sourceClFile: z.string(),
  className: z.string(),
  classCode: z.string().nullable().optional(),
  markSetDefs: z.array(
    z.object({
      filePrefix: z.string(),
      code: z.string(),
      description: z.string(),
      weight: z.number(),
      sortOrder: z.number()
    })
  ),
  students: z.object({
    incoming: z.number(),
    matched: z.number(),
    new: z.number(),
    ambiguous: z.number(),
    localOnly: z.number()
  }),
  markSets: z.object({
    incoming: z.number(),
    matched: z.number(),
    new: z.number()
  }),
  warnings: z.array(z.record(z.string(), z.unknown())).optional()
});

export const ClassesUpdateFromLegacyResultSchema = z.object({
  ok: z.literal(true),
  classId: z.string(),
  students: z.object({
    matched: z.number(),
    created: z.number(),
    updated: z.number(),
    localOnly: z.number(),
    ambiguousSkipped: z.number()
  }),
  markSets: z.object({
    matched: z.number(),
    created: z.number(),
    undeleted: z.number()
  }),
  assessments: z.object({
    matched: z.number(),
    created: z.number(),
    updated: z.number()
  }),
  scores: z.object({
    upserted: z.number()
  }),
  warnings: z.array(z.record(z.string(), z.unknown())).optional(),
  sourceClFile: z.string(),
  importedMarkFiles: z.array(z.string()).optional()
});

export const ClassesImportLinkGetResultSchema = z.object({
  classId: z.string(),
  legacyClassFolderPath: z.string().nullable(),
  legacyClFile: z.string().nullable().optional(),
  legacyYearToken: z.string().nullable().optional(),
  lastImportedAt: z.string().nullable().optional()
});

export const ClassesImportLinkSetResultSchema = z.object({
  ok: z.literal(true),
  classId: z.string(),
  legacyClassFolderPath: z.string()
});

export const ClassesUpdateFromAttachedLegacyResultSchema =
  ClassesUpdateFromLegacyResultSchema;

export const MarkSetsListResultSchema = z.object({
  markSets: z.array(
    z.object({
      id: z.string(),
      code: z.string(),
      description: z.string(),
      sortOrder: z.number(),
      isDefault: z.boolean().optional(),
      deletedAt: z.string().nullable().optional()
    })
  )
});

export const MarkSetsCreateResultSchema = z.object({
  markSetId: z.string()
});

export const MarkSetsDeleteResultSchema = z.object({
  ok: z.literal(true)
});

export const MarkSetsUndeleteResultSchema = z.object({
  ok: z.literal(true)
});

export const MarkSetsSetDefaultResultSchema = z.object({
  ok: z.literal(true)
});

export const MarkSetsCloneResultSchema = z.object({
  markSetId: z.string()
});

export const MarkSetsTransferPreviewResultSchema = z.object({
  sourceAssessmentCount: z.number(),
  candidateCount: z.number(),
  collisions: z.array(
    z.object({
      sourceAssessmentId: z.string(),
      sourceIdx: z.number(),
      sourceTitle: z.string(),
      targetAssessmentId: z.string(),
      targetIdx: z.number(),
      key: z.string()
    })
  ),
  studentAlignment: z.object({
    sourceRows: z.number(),
    targetRows: z.number(),
    alignedRows: z.number()
  })
});

export const MarkSetsTransferApplyResultSchema = z.object({
  ok: z.literal(true),
  assessments: z.object({
    created: z.number(),
    merged: z.number()
  }),
  scores: z.object({
    upserted: z.number()
  }),
  remarks: z.object({
    upserted: z.number()
  }),
  warnings: z.array(z.record(z.string(), z.unknown())).optional()
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
  updated: z.number(),
  rejected: z.number().optional(),
  limitExceeded: z.boolean().optional(),
  errors: z
    .array(
      z.object({
        row: z.number(),
        col: z.number(),
        code: z.string(),
        message: z.string()
      })
    )
    .optional()
});

export const EntriesDeleteResultSchema = z.object({
  ok: z.literal(true)
});

export const EntriesCloneSaveResultSchema = z.object({
  ok: z.literal(true),
  clone: z.object({
    sourceMarkSetId: z.string().nullable(),
    title: z.string().nullable()
  })
});

export const EntriesClonePeekResultSchema = z.object({
  clone: z.object({
    exists: z.boolean(),
    sourceMarkSetId: z.string().optional().nullable(),
    title: z.string().optional().nullable()
  })
});

export const EntriesCloneApplyResultSchema = z.object({
  ok: z.literal(true),
  assessmentId: z.string()
});

export const MarksPrefHideDeletedGetResultSchema = z.object({
  hideDeleted: z.boolean()
});

export const MarksPrefHideDeletedSetResultSchema = z.object({
  ok: z.literal(true)
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
  cells: z.array(z.array(z.number().nullable())),
  filters: z
    .object({
      term: z.number().nullable(),
      categoryName: z.string().nullable(),
      typesMask: z.number().nullable()
    })
    .optional(),
  studentScope: z.enum(["all", "active", "valid"]).optional()
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

export const StudentsMembershipBulkSetResultSchema = z.object({
  ok: z.literal(true),
  updated: z.number(),
  failed: z
    .array(
      z.object({
        studentId: z.string(),
        code: z.string(),
        message: z.string()
      })
    )
    .optional()
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
      outOf: z.number().nullable(),
      isDeletedLike: z.boolean().optional()
    })
  )
});

export const AssessmentsCreateResultSchema = z.object({
  assessmentId: z.string()
});

export const AssessmentsBulkCreateResultSchema = z.object({
  ok: z.literal(true),
  created: z.number(),
  assessmentIds: z.array(z.string())
});

export const AssessmentsUpdateResultSchema = z.object({
  ok: z.literal(true)
});

export const AssessmentsBulkUpdateResultSchema = z.object({
  ok: z.literal(true),
  updated: z.number(),
  rejected: z.number(),
  errors: z
    .array(
      z.object({
        index: z.number(),
        assessmentId: z.string().optional(),
        code: z.string(),
        message: z.string()
      })
    )
    .optional()
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
    calcMethod: z.number(),
    isDefault: z.boolean().optional(),
    deletedAt: z.string().nullable().optional(),
    blockTitle: z.string().nullable().optional()
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

const AnalyticsStudentScopeSchema = z.enum(["all", "active", "valid"]);

export const AnalyticsFiltersOptionsResultSchema = z.object({
  terms: z.array(z.number()),
  categories: z.array(z.string()),
  types: z.array(
    z.object({
      bit: z.number(),
      key: z.string(),
      label: z.string()
    })
  ),
  studentScopes: z.array(AnalyticsStudentScopeSchema)
});

export const AnalyticsClassOpenResultSchema = z.object({
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
  studentScope: AnalyticsStudentScopeSchema,
  kpis: z.object({
    classAverage: z.number().nullable(),
    classMedian: z.number().nullable(),
    studentCount: z.number(),
    finalMarkCount: z.number(),
    noMarkRate: z.number(),
    zeroRate: z.number()
  }),
  distributions: z.object({
    bins: z.array(
      z.object({
        label: z.string(),
        min: z.number(),
        max: z.number(),
        count: z.number()
      })
    ),
    noFinalMarkCount: z.number()
  }),
  perAssessment: z.array(CalcPerAssessmentSchema),
  perCategory: z.array(CalcPerCategorySchema),
  topBottom: z.object({
    top: z.array(CalcPerStudentSchema),
    bottom: z.array(CalcPerStudentSchema)
  }),
  rows: z.array(CalcPerStudentSchema)
});

export const AnalyticsClassRowsResultSchema = z.object({
  rows: z.array(CalcPerStudentSchema),
  totalRows: z.number(),
  page: z.number(),
  pageSize: z.number(),
  sortBy: z.string(),
  sortDir: z.enum(["asc", "desc"]),
  appliedCohort: z.object({
    finalMin: z.number().nullable(),
    finalMax: z.number().nullable(),
    includeNoFinal: z.boolean()
  })
});

export const AnalyticsClassAssessmentDrilldownResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  markSet: z.object({
    id: z.string(),
    code: z.string(),
    description: z.string()
  }),
  filters: z.object({
    term: z.number().nullable(),
    categoryName: z.string().nullable(),
    typesMask: z.number().nullable()
  }),
  studentScope: AnalyticsStudentScopeSchema,
  assessment: z.object({
    assessmentId: z.string(),
    idx: z.number(),
    date: z.string().nullable(),
    categoryName: z.string().nullable(),
    title: z.string(),
    term: z.number().nullable(),
    legacyType: z.number().nullable(),
    weight: z.number(),
    outOf: z.number()
  }),
  rows: z.array(
    z.object({
      studentId: z.string(),
      displayName: z.string(),
      sortOrder: z.number(),
      active: z.boolean(),
      status: z.enum(["no_mark", "zero", "scored"]),
      raw: z.number().nullable(),
      percent: z.number().nullable(),
      finalMark: z.number().nullable()
    })
  ),
  totalRows: z.number(),
  page: z.number(),
  pageSize: z.number(),
  sortBy: z.string(),
  sortDir: z.enum(["asc", "desc"]),
  classStats: CalcPerAssessmentSchema
});

export const AnalyticsStudentOpenResultSchema = z.object({
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
  studentScope: AnalyticsStudentScopeSchema.optional(),
  student: CalcPerStudentSchema,
  finalMark: z.number().nullable(),
  counts: z.object({
    noMark: z.number(),
    zero: z.number(),
    scored: z.number()
  }),
  categoryBreakdown: z.array(
    z.object({
      name: z.string(),
      value: z.number().nullable(),
      weight: z.number(),
      hasData: z.boolean()
    })
  ),
  assessmentTrail: z.array(
    z.object({
      assessmentId: z.string(),
      idx: z.number(),
      title: z.string(),
      date: z.string().nullable(),
      categoryName: z.string().nullable(),
      term: z.number().nullable(),
      legacyType: z.number().nullable(),
      weight: z.number(),
      outOf: z.number(),
      status: z.enum(["no_mark", "zero", "scored"]),
      score: z.number().nullable(),
      percent: z.number().nullable(),
      classAvgRaw: z.number().nullable().optional(),
      classAvgPercent: z.number().nullable().optional()
    })
  ),
  attendanceSummary: z
    .object({
      monthsWithData: z.number(),
      codedDays: z.number()
    })
    .optional()
});

export const AnalyticsStudentCompareResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  markSet: z.object({
    id: z.string(),
    code: z.string(),
    description: z.string()
  }),
  filters: z.object({
    term: z.number().nullable(),
    categoryName: z.string().nullable(),
    typesMask: z.number().nullable()
  }),
  studentScope: AnalyticsStudentScopeSchema,
  student: CalcPerStudentSchema,
  cohort: z.object({
    studentCount: z.number(),
    finalMarkCount: z.number(),
    classAverage: z.number().nullable(),
    classMedian: z.number().nullable()
  }),
  finalMarkDelta: z.number().nullable(),
  percentile: z.number().nullable(),
  categoryComparison: z.array(
    z.object({
      name: z.string(),
      weight: z.number(),
      studentValue: z.number().nullable(),
      classAvg: z.number().nullable(),
      hasData: z.boolean()
    })
  ),
  assessmentComparison: z.array(
    z.object({
      assessmentId: z.string(),
      idx: z.number(),
      title: z.string(),
      date: z.string().nullable(),
      categoryName: z.string().nullable(),
      term: z.number().nullable(),
      legacyType: z.number().nullable(),
      weight: z.number(),
      outOf: z.number(),
      status: z.enum(["no_mark", "zero", "scored"]),
      raw: z.number().nullable(),
      percent: z.number().nullable(),
      classAvgRaw: z.number().nullable(),
      classAvgPercent: z.number().nullable(),
      classMedianPercent: z.number().nullable()
    })
  )
});

export const AnalyticsStudentTrendResultSchema = z.object({
  student: z.object({
    id: z.string(),
    displayName: z.string(),
    active: z.boolean()
  }),
  filters: z.object({
    term: z.number().nullable(),
    categoryName: z.string().nullable(),
    typesMask: z.number().nullable()
  }),
  points: z.array(
    z.object({
      markSetId: z.string(),
      code: z.string(),
      sortOrder: z.number(),
      finalMark: z.number().nullable(),
      classAverage: z.number().nullable(),
      classMedian: z.number().nullable(),
      validForSet: z.boolean()
    })
  ),
  summary: z.object({
    selectedMarkSetCount: z.number(),
    finalMarkCount: z.number(),
    averageFinal: z.number().nullable(),
    bestFinal: z.number().nullable(),
    worstFinal: z.number().nullable()
  })
});

const AnalyticsCombinedMarkSetSchema = z.object({
  id: z.string(),
  code: z.string(),
  description: z.string(),
  sortOrder: z.number(),
  weight: z.number()
});

const AnalyticsCombinedRowSchema = z.object({
  studentId: z.string(),
  displayName: z.string(),
  sortOrder: z.number(),
  active: z.boolean(),
  combinedFinal: z.number().nullable(),
  perMarkSet: z.array(
    z.object({
      markSetId: z.string(),
      code: z.string(),
      description: z.string(),
      weight: z.number(),
      valid: z.boolean(),
      finalMark: z.number().nullable()
    })
  )
});

export const AnalyticsCombinedOptionsResultSchema = z.object({
  markSets: z.array(
    AnalyticsCombinedMarkSetSchema.extend({
      deletedAt: z.string().nullable().optional()
    })
  ),
  terms: z.array(z.number()),
  categories: z.array(z.string()),
  types: z.array(
    z.object({
      bit: z.number(),
      key: z.string(),
      label: z.string()
    })
  ),
  studentScopes: z.array(AnalyticsStudentScopeSchema)
});

export const AnalyticsCombinedOpenResultSchema = z.object({
  class: z.object({
    id: z.string(),
    name: z.string()
  }),
  markSets: z.array(AnalyticsCombinedMarkSetSchema),
  filters: z.object({
    term: z.number().nullable(),
    categoryName: z.string().nullable(),
    typesMask: z.number().nullable()
  }),
  studentScope: AnalyticsStudentScopeSchema,
  settingsApplied: z
    .object({
      combineMethod: z.string(),
      fallbackUsedCount: z.number()
    })
    .optional(),
  kpis: z.object({
    classAverage: z.number().nullable(),
    classMedian: z.number().nullable(),
    studentCount: z.number(),
    finalMarkCount: z.number(),
    noCombinedFinalCount: z.number()
  }),
  distributions: z.object({
    bins: z.array(
      z.object({
        label: z.string(),
        min: z.number(),
        max: z.number(),
        count: z.number()
      })
    ),
    noCombinedFinalCount: z.number()
  }),
  perMarkSet: z.array(
    z.object({
      markSetId: z.string(),
      code: z.string(),
      description: z.string(),
      weight: z.number(),
      finalMarkCount: z.number(),
      classAverage: z.number().nullable(),
      classMedian: z.number().nullable()
    })
  ),
  rows: z.array(AnalyticsCombinedRowSchema),
  topBottom: z.object({
    top: z.array(AnalyticsCombinedRowSchema),
    bottom: z.array(AnalyticsCombinedRowSchema)
  })
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

export const PlannerUnitSchema = z.object({
  id: z.string(),
  sortOrder: z.number(),
  title: z.string(),
  startDate: z.string().nullable(),
  endDate: z.string().nullable(),
  summary: z.string(),
  expectations: z.array(z.string()),
  resources: z.array(z.string()),
  archived: z.boolean(),
  createdAt: z.string(),
  updatedAt: z.string()
});

export const PlannerUnitsListResultSchema = z.object({
  units: z.array(PlannerUnitSchema)
});

export const PlannerUnitsOpenResultSchema = z.object({
  unit: PlannerUnitSchema
});

export const PlannerUnitsCreateResultSchema = z.object({
  unitId: z.string()
});

export const PlannerUnitsUpdateResultSchema = z.object({
  ok: z.literal(true)
});

export const PlannerUnitsReorderResultSchema = z.object({
  ok: z.literal(true)
});

export const PlannerUnitsArchiveResultSchema = z.object({
  ok: z.literal(true)
});

export const PlannerLessonSchema = z.object({
  id: z.string(),
  unitId: z.string().nullable(),
  sortOrder: z.number(),
  lessonDate: z.string().nullable(),
  title: z.string(),
  outline: z.string(),
  detail: z.string(),
  followUp: z.string(),
  homework: z.string(),
  durationMinutes: z.number().nullable(),
  archived: z.boolean(),
  createdAt: z.string(),
  updatedAt: z.string()
});

export const PlannerLessonsListResultSchema = z.object({
  lessons: z.array(PlannerLessonSchema)
});

export const PlannerLessonsOpenResultSchema = z.object({
  lesson: PlannerLessonSchema
});

export const PlannerLessonsCreateResultSchema = z.object({
  lessonId: z.string()
});

export const PlannerLessonsUpdateResultSchema = z.object({
  ok: z.literal(true)
});

export const PlannerLessonsReorderResultSchema = z.object({
  ok: z.literal(true)
});

export const PlannerLessonsArchiveResultSchema = z.object({
  ok: z.literal(true)
});

export const PlannerPublishedArtifactSchema = z.object({
  id: z.string(),
  artifactKind: z.enum(["unit", "lesson", "course_description", "time_management"]),
  sourceId: z.string().nullable(),
  title: z.string(),
  status: z.enum(["draft", "published", "archived"]),
  version: z.number(),
  model: z.unknown(),
  createdAt: z.string(),
  updatedAt: z.string()
});

export const PlannerPublishListResultSchema = z.object({
  published: z.array(PlannerPublishedArtifactSchema)
});

export const PlannerPublishPreviewResultSchema = z.object({
  artifactKind: z.enum(["unit", "lesson", "course_description", "time_management"]),
  sourceId: z.string().nullable().optional(),
  title: z.string(),
  model: z.unknown()
});

export const PlannerPublishCommitResultSchema = z.object({
  ok: z.literal(true),
  publishId: z.string(),
  status: z.enum(["draft", "published", "archived"]),
  version: z.number(),
  settingsApplied: z.unknown().optional()
});

export const PlannerPublishUpdateStatusResultSchema = z.object({
  ok: z.literal(true)
});

export const CourseDescriptionProfileSchema = z.object({
  courseTitle: z.string(),
  gradeLabel: z.string(),
  periodMinutes: z.number(),
  periodsPerWeek: z.number(),
  totalWeeks: z.number(),
  strands: z.array(z.string()),
  policyText: z.string(),
  updatedAt: z.string().nullable()
});

export const CourseDescriptionGetProfileResultSchema = z.object({
  classId: z.string(),
  profile: CourseDescriptionProfileSchema
});

export const CourseDescriptionUpdateProfileResultSchema = z.object({
  ok: z.literal(true)
});

export const CourseDescriptionModelResultSchema = z.object({
  class: z.object({ id: z.string(), name: z.string() }),
  profile: z.object({
    courseTitle: z.string(),
    gradeLabel: z.string(),
    periodMinutes: z.number(),
    periodsPerWeek: z.number(),
    totalWeeks: z.number(),
    strands: z.array(z.unknown()),
    policyText: z.string()
  }),
  schedule: z.object({
    periodMinutes: z.number(),
    periodsPerWeek: z.number(),
    totalWeeks: z.number(),
    totalHours: z.number()
  }),
  units: z.array(z.unknown()),
  lessons: z.array(z.unknown()),
  generatedAt: z.string(),
  settingsApplied: z.unknown().optional()
});

export const TimeManagementModelResultSchema = z.object({
  class: z.object({ id: z.string(), name: z.string() }),
  inputs: z.object({
    periodMinutes: z.number(),
    periodsPerWeek: z.number(),
    totalWeeks: z.number(),
    includeArchived: z.boolean()
  }),
  totals: z.object({
    plannedMinutes: z.number(),
    availableMinutes: z.number(),
    remainingMinutes: z.number(),
    utilizationPercent: z.number()
  }),
  generatedAt: z.string(),
  settingsApplied: z.unknown().optional()
});

export const SetupGetResultSchema = z.object({
  analysis: z.object({
    defaultStudentScope: z.enum(["all", "active", "valid"]),
    showInactiveStudents: z.boolean(),
    showDeletedEntries: z.boolean(),
    histogramBins: z.number(),
    defaultSortBy: z.enum(["sortOrder", "displayName", "finalMark"]),
    defaultTopBottomCount: z.number()
  }),
  attendance: z.object({
    schoolYearStartMonth: z.number(),
    presentCode: z.string(),
    absentCode: z.string(),
    lateCode: z.string(),
    excusedCode: z.string(),
    tardyThresholdMinutes: z.number()
  }),
  comments: z.object({
    defaultTransferPolicy: z.enum(["replace", "append", "fill_blank", "source_if_longer"]),
    appendSeparator: z.string(),
    enforceFit: z.boolean(),
    enforceMaxChars: z.boolean(),
    defaultMaxChars: z.number()
  }),
  printer: z.object({
    fontScale: z.number(),
    landscapeWideTables: z.boolean(),
    repeatHeaders: z.boolean(),
    showGeneratedAt: z.boolean(),
    defaultMarginMm: z.number()
  }),
  integrations: z.object({
    defaultSisProfile: z.enum(["mb_exchange_v1", "sis_roster_v1", "sis_marks_v1"]),
    defaultMatchMode: z.enum(["student_no_then_name", "name_only", "sort_order"]),
    defaultCollisionPolicy: z.enum(["merge_existing", "append_new", "stop_on_collision"]),
    autoPreviewBeforeApply: z.boolean(),
    adminTransferDefaultPolicy: z.enum(["replace", "append", "fill_blank", "source_if_longer"])
  }),
  marks: z.object({
    defaultHideDeletedEntries: z.boolean(),
    defaultAutoPreviewBeforeBulkApply: z.boolean()
  }),
  exchange: z.object({
    defaultExportStudentScope: z.enum(["all", "active", "valid"]),
    includeStateColumnsByDefault: z.boolean()
  }),
  analytics: z.object({
    defaultPageSize: z.number(),
    defaultCohortMode: z.enum(["none", "bin", "threshold"])
  }),
  planner: z.object({
    defaultLessonDurationMinutes: z.number(),
    defaultPublishStatus: z.enum(["draft", "published", "archived"]),
    showArchivedByDefault: z.boolean(),
    defaultUnitTitlePrefix: z.string()
  }),
  courseDescription: z.object({
    defaultPeriodMinutes: z.number(),
    defaultPeriodsPerWeek: z.number(),
    defaultTotalWeeks: z.number(),
    includePolicyByDefault: z.boolean()
  }),
  reports: z.object({
    plannerHeaderStyle: z.enum(["compact", "classic", "minimal"]),
    showGeneratedAt: z.boolean(),
    defaultStudentScope: z.enum(["all", "active", "valid"]),
    defaultAnalyticsScope: z.enum(["all", "active", "valid"]),
    showFiltersInHeaderByDefault: z.boolean()
  }),
  security: z.object({
    passwordEnabled: z.boolean(),
    passwordHint: z.string().nullable(),
    confirmDeletes: z.boolean(),
    autoLockMinutes: z.number()
  }),
  email: z.object({
    enabled: z.boolean(),
    fromName: z.string(),
    replyTo: z.string(),
    subjectPrefix: z.string(),
    defaultCc: z.string()
  })
});

export const SetupUpdateResultSchema = z.object({
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

export const CommentsRemarksUpsertOneResultSchema = z.object({
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

export const CommentsTransferPreviewResultSchema = z.object({
  counts: z.object({
    sourceRows: z.number(),
    targetRows: z.number(),
    matched: z.number(),
    unmatchedSource: z.number(),
    unmatchedTarget: z.number(),
    same: z.number(),
    different: z.number(),
    sourceOnly: z.number(),
    targetOnly: z.number()
  }),
  rows: z.array(
    z.object({
      sourceStudentId: z.string().optional(),
      targetStudentId: z.string().optional(),
      sourceDisplayName: z.string().optional(),
      targetDisplayName: z.string().optional(),
      sourceRemark: z.string(),
      targetRemark: z.string(),
      status: z.enum(["same", "different", "source_only", "target_only", "unmatched"])
    })
  )
});

export const CommentsTransferApplyResultSchema = z.object({
  ok: z.literal(true),
  updated: z.number(),
  skipped: z.number(),
  unchanged: z.number(),
  warnings: z.array(z.record(z.string(), z.unknown())).optional()
});

export const CommentsTransferFloodFillResultSchema = z.object({
  ok: z.literal(true),
  updated: z.number(),
  skipped: z.number()
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
  perAssessment: z.array(CalcPerAssessmentSchema),
  studentScope: z.enum(["all", "active", "valid"]).optional()
});

export const ReportsCombinedAnalysisModelResultSchema = AnalyticsCombinedOpenResultSchema;
export const ReportsClassAssessmentDrilldownModelResultSchema =
  AnalyticsClassAssessmentDrilldownResultSchema;
export const ReportsPlannerUnitModelResultSchema = z.object({
  artifactKind: z.literal("unit"),
  title: z.string(),
  unit: z.object({
    unitId: z.string(),
    title: z.string(),
    startDate: z.string().nullable(),
    endDate: z.string().nullable(),
    summary: z.string(),
    expectations: z.array(z.string()),
    resources: z.array(z.string())
  }),
  lessons: z.array(
    z.object({
      lessonId: z.string(),
      title: z.string(),
      lessonDate: z.string().nullable(),
      outline: z.string(),
      detail: z.string(),
      followUp: z.string(),
      homework: z.string(),
      durationMinutes: z.number().nullable()
    })
  )
});
export const ReportsPlannerLessonModelResultSchema = z.object({
  artifactKind: z.literal("lesson"),
  title: z.string(),
  lesson: z.object({
    lessonId: z.string(),
    unitId: z.string().nullable(),
    title: z.string(),
    lessonDate: z.string().nullable(),
    outline: z.string(),
    detail: z.string(),
    followUp: z.string(),
    homework: z.string(),
    durationMinutes: z.number().nullable(),
    unitTitle: z.string().nullable()
  })
});
export const ReportsCourseDescriptionModelResultSchema = CourseDescriptionModelResultSchema;
export const ReportsTimeManagementModelResultSchema = TimeManagementModelResultSchema;

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
  studentScope: z.enum(["all", "active", "valid"]).optional(),
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

export const ExchangeWarningSchema = z.object({
  line: z.number().optional(),
  code: z.string(),
  message: z.string()
});

export const ExchangePreviewClassCsvResultSchema = z.object({
  ok: z.literal(true),
  path: z.string(),
  mode: z.string(),
  rowsTotal: z.number(),
  rowsParsed: z.number(),
  rowsMatched: z.number(),
  rowsUnmatched: z.number(),
  warningsCount: z.number(),
  warnings: z.array(ExchangeWarningSchema),
  previewRows: z.array(
    z.object({
      line: z.number(),
      studentId: z.string(),
      markSetCode: z.string(),
      assessmentIdx: z.number(),
      status: z.string()
    })
  )
});

export const ExchangeApplyClassCsvResultSchema = z.object({
  ok: z.literal(true),
  updated: z.number(),
  rowsTotal: z.number(),
  rowsParsed: z.number(),
  skipped: z.number(),
  warningsCount: z.number(),
  warnings: z.array(ExchangeWarningSchema),
  mode: z.string(),
  path: z.string()
});

export const ExchangeImportClassCsvResultSchema = z.object({
  ok: z.literal(true),
  updated: z.number(),
  rowsTotal: z.number().optional(),
  rowsParsed: z.number().optional(),
  skipped: z.number().optional(),
  warningsCount: z.number().optional(),
  warnings: z.array(ExchangeWarningSchema).optional(),
  mode: z.string().optional(),
  path: z.string().optional()
});

export const IntegrationsSisPreviewImportResultSchema = z.object({
  ok: z.literal(true),
  classId: z.string(),
  path: z.string(),
  profile: z.string(),
  matchMode: z.string(),
  mode: z.string(),
  rowsTotal: z.number(),
  rowsParsed: z.number(),
  matched: z.number(),
  new: z.number(),
  ambiguous: z.number(),
  invalid: z.number(),
  warnings: z.array(z.record(z.string(), z.unknown())),
  previewRows: z.array(z.record(z.string(), z.unknown()))
});

export const IntegrationsSisApplyImportResultSchema = z.object({
  ok: z.literal(true),
  classId: z.string(),
  path: z.string(),
  profile: z.string(),
  matchMode: z.string(),
  mode: z.string(),
  collisionPolicy: z.string(),
  created: z.number(),
  updated: z.number(),
  ambiguousSkipped: z.number(),
  warnings: z.array(z.record(z.string(), z.unknown()))
});

export const IntegrationsSisExportRosterResultSchema = z.object({
  ok: z.literal(true),
  rowsExported: z.number(),
  profile: z.string(),
  path: z.string()
});

export const IntegrationsSisExportMarksResultSchema = z.object({
  ok: z.literal(true),
  rowsExported: z.number(),
  assessmentsExported: z.number(),
  profile: z.string(),
  path: z.string()
});

export const IntegrationsAdminTransferPreviewPackageResultSchema = z.object({
  metadata: z.record(z.string(), z.unknown()),
  markSetCount: z.number(),
  studentAlignment: z.object({
    sourceRows: z.number(),
    targetRows: z.number(),
    matched: z.number(),
    unmatchedSource: z.number(),
    ambiguous: z.number()
  }),
  collisions: z.array(z.record(z.string(), z.unknown())),
  warnings: z.array(z.record(z.string(), z.unknown()))
});

export const IntegrationsAdminTransferApplyPackageResultSchema = z.object({
  ok: z.literal(true),
  students: z.object({
    created: z.number()
  }),
  assessments: z.object({
    created: z.number(),
    merged: z.number()
  }),
  scores: z.object({
    upserted: z.number()
  }),
  remarks: z.object({
    updated: z.number()
  }),
  warnings: z.array(z.record(z.string(), z.unknown()))
});

export const IntegrationsAdminTransferExportPackageResultSchema = z.object({
  ok: z.literal(true),
  entriesWritten: z.number(),
  path: z.string(),
  format: z.literal("mb-admin-transfer-v1")
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
