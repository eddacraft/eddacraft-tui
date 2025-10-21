/**
 * Parser for official GitHub Spec-Kit spec.md format
 *
 * Spec.md focuses on WHAT and WHY (not HOW):
 * - Feature metadata
 * - User Scenarios & Testing (prioritized user stories)
 * - Requirements (functional requirements, key entities)
 * - Success Criteria (measurable outcomes)
 */

interface SpecMetadata {
  feature?: string;
  branch?: string;
  date?: string;
  status?: string;
  [key: string]: unknown;
}

interface UserScenario {
  priority: 'P1' | 'P2' | 'P3' | string;
  title: string;
  asA: string;
  iWantTo: string;
  soThat: string;
  acceptanceScenarios: string[];
  edgeCases: string[];
}

interface FunctionalRequirement {
  code: string;
  description: string;
  needsClarification: boolean;
  clarificationQuestion?: string;
}

interface EntityDefinition {
  name: string;
  represents: string;
  keyAttributes: string[];
  relationships: string[];
}

interface SuccessCriteria {
  quantitative: string[];
  qualitative: string[];
  security?: string[];
  performance?: string[];
}

export interface ParsedSpec {
  metadata: SpecMetadata;
  userScenarios: UserScenario[];
  requirements: {
    functional: FunctionalRequirement[];
    entities: EntityDefinition[];
  };
  successCriteria: SuccessCriteria;
  clarifications: string[]; // All [NEEDS CLARIFICATION] markers
}

export class SpecParser {
  parseSpec(content: string): ParsedSpec {
    const result: ParsedSpec = {
      metadata: {},
      userScenarios: [],
      requirements: {
        functional: [],
        entities: [],
      },
      successCriteria: {
        quantitative: [],
        qualitative: [],
      },
      clarifications: [],
    };

    // Extract metadata from first section (before ## headers)
    result.metadata = this.extractMetadata(content);

    // Parse user scenarios section
    result.userScenarios = this.parseUserScenarios(content);

    // Parse requirements section
    result.requirements = this.parseRequirements(content);

    // Parse success criteria
    result.successCriteria = this.parseSuccessCriteria(content);

    // Extract all clarifications
    result.clarifications = this.extractClarifications(content);

    return result;
  }

  private extractMetadata(content: string): SpecMetadata {
    const metadata: SpecMetadata = {};

    // Extract title (# Feature: ...)
    const titleMatch = content.match(/^#\s+Feature:\s+(.+)$/m);
    if (titleMatch) {
      metadata.feature = titleMatch[1].trim();
    }

    // Extract bold key-value pairs (** Key **: value)
    // Stop at next ** or newline to handle multiple metadata on same line
    const metadataRegex = /\*\*([^*]+)\*\*:\s*`?([^`*\n]+?)`?\s*(?=\*\*|\n|$)/g;
    let match;
    while ((match = metadataRegex.exec(content)) !== null) {
      const key = match[1].trim().toLowerCase().replace(/\s+/g, '_');
      const value = match[2].trim();
      metadata[key] = value;
    }

    return metadata;
  }

  private parseUserScenarios(content: string): UserScenario[] {
    const scenarios: UserScenario[] = [];

    // Find "User Scenarios & Testing" section
    const scenarioSectionMatch = content.match(
      /##\s+User Scenarios?\s*(?:&|and)?\s*Testing([\s\S]*?)(?=\n##\s|\n#\s|$)/i
    );

    if (!scenarioSectionMatch) {
      return scenarios;
    }

    const scenarioSection = scenarioSectionMatch[1];

    // Split by ### headers (each scenario)
    const scenarioBlocks = scenarioSection.split(/###\s+/).slice(1);

    for (const block of scenarioBlocks) {
      const scenario = this.parseUserScenario(block);
      if (scenario) {
        scenarios.push(scenario);
      }
    }

    return scenarios;
  }

  private parseUserScenario(block: string): UserScenario | null {
    // Extract priority and title from first line (P1: Title)
    const titleMatch = block.match(/^(P\d+):\s+(.+)$/m);
    if (!titleMatch) {
      return null;
    }

    const priority = titleMatch[1];
    const title = titleMatch[2].trim();

    // Extract user story components (handle multiline with line breaks)
    const asAMatch = block.match(/\*\*As a\*\*\s+(.+?)(?=\s*\*\*|\n\n|$)/is);
    const iWantToMatch = block.match(/\*\*I want(?:\s+to)?\*\*\s+(.+?)(?=\s*\*\*|\n\n|$)/is);
    const soThatMatch = block.match(/\*\*So(?:\s+|\n)that\*\*\s+(.+?)(?=\s*\n\n|$)/is);

    // Extract acceptance scenarios
    const acceptanceScenarios: string[] = [];
    const acceptanceMatch = block.match(
      /\*\*Acceptance Scenarios:\*\*([\s\S]*?)(?=\*\*Edge Cases:|\*\*\[NEEDS|###|$)/i
    );
    if (acceptanceMatch) {
      const scenarios = acceptanceMatch[1]
        .split(/\n[-*]\s+/)
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
      acceptanceScenarios.push(...scenarios);
    }

    // Extract edge cases
    const edgeCases: string[] = [];
    const edgeCaseMatch = block.match(/\*\*Edge Cases:\*\*([\s\S]*?)(?=###|$)/i);
    if (edgeCaseMatch) {
      const cases = edgeCaseMatch[1]
        .split(/\n[-*]\s+/)
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
      edgeCases.push(...cases);
    }

    return {
      priority,
      title,
      asA: asAMatch?.[1]?.trim() || '',
      iWantTo: iWantToMatch?.[1]?.trim() || '',
      soThat: soThatMatch?.[1]?.trim() || '',
      acceptanceScenarios,
      edgeCases,
    };
  }

  private parseRequirements(content: string): {
    functional: FunctionalRequirement[];
    entities: EntityDefinition[];
  } {
    const requirements = {
      functional: [] as FunctionalRequirement[],
      entities: [] as EntityDefinition[],
    };

    // Find "Requirements" section
    const reqSectionMatch = content.match(/##\s+Requirements([\s\S]*?)(?=\n##\s|\n#\s|$)/i);

    if (!reqSectionMatch) {
      return requirements;
    }

    const reqSection = reqSectionMatch[1];

    // Parse functional requirements
    requirements.functional = this.parseFunctionalRequirements(reqSection);

    // Parse entity definitions
    requirements.entities = this.parseEntities(reqSection);

    return requirements;
  }

  private parseFunctionalRequirements(section: string): FunctionalRequirement[] {
    const requirements: FunctionalRequirement[] = [];

    // Find "Functional Requirements" subsection
    const functionalMatch = section.match(/###\s+Functional Requirements([\s\S]*?)(?=\n###|$)/i);

    if (!functionalMatch) {
      return requirements;
    }

    const functionalSection = functionalMatch[1];

    // Match FR-XXX: Description or [NEEDS CLARIFICATION: question]
    const reqRegex = /\*\*(FR-\d+)\*\*:\s+(.+?)(?=\n\*\*FR-|\n###|$)/gs;
    let match;

    while ((match = reqRegex.exec(functionalSection)) !== null) {
      const code = match[1];
      const description = match[2].trim();

      // Check if this is a clarification request
      const clarificationMatch = description.match(/\[NEEDS CLARIFICATION:\s*(.+?)\]/);

      requirements.push({
        code,
        description: clarificationMatch ? description : description,
        needsClarification: !!clarificationMatch,
        clarificationQuestion: clarificationMatch?.[1]?.trim(),
      });
    }

    return requirements;
  }

  private parseEntities(section: string): EntityDefinition[] {
    const entities: EntityDefinition[] = [];

    // Find "Key Entities" subsection
    const entitiesMatch = section.match(/###\s+Key Entities([\s\S]*?)(?=\n##|$)/i);

    if (!entitiesMatch) {
      return entities;
    }

    const entitiesSection = entitiesMatch[1];

    // Split by ** EntityName **
    const entityBlocks = entitiesSection.split(/\*\*([^*]+)\*\*/g).slice(1);

    for (let i = 0; i < entityBlocks.length; i += 2) {
      const entityName = entityBlocks[i].trim();
      const entityContent = entityBlocks[i + 1] || '';

      if (!entityName || !entityContent.trim()) {
        continue;
      }

      const entity = this.parseEntity(entityName, entityContent);
      if (entity) {
        entities.push(entity);
      }
    }

    return entities;
  }

  private parseEntity(name: string, content: string): EntityDefinition | null {
    const representsMatch = content.match(/[-*]\s+Represents:\s+(.+)/i);
    const attributesMatch = content.match(/[-*]\s+Key Attributes:\s+(.+)/i);
    const relationshipsMatch = content.match(/[-*]\s+Relationships:\s+(.+)/i);

    return {
      name,
      represents: representsMatch?.[1]?.trim() || '',
      keyAttributes:
        attributesMatch?.[1]
          ?.split(',')
          .map((a) => a.trim())
          .filter((a) => a.length > 0) || [],
      relationships:
        relationshipsMatch?.[1]
          ?.split(',')
          .map((r) => r.trim())
          .filter((r) => r.length > 0) || [],
    };
  }

  private parseSuccessCriteria(content: string): SuccessCriteria {
    const criteria: SuccessCriteria = {
      quantitative: [],
      qualitative: [],
    };

    // Find "Success Criteria" section
    const criteriaSectionMatch = content.match(
      /##\s+Success Criteria([\s\S]*?)(?=\n##\s|\n#\s|$)/i
    );

    if (!criteriaSectionMatch) {
      return criteria;
    }

    const criteriaSection = criteriaSectionMatch[1];

    // Parse quantitative metrics
    const quantMatch = criteriaSection.match(/###\s+Quantitative Metrics([\s\S]*?)(?=\n###|$)/i);
    if (quantMatch) {
      const metrics = quantMatch[1]
        .split(/\n[-*]\s+/)
        .map((m) => m.trim())
        .filter((m) => m.length > 0);
      criteria.quantitative = metrics;
    }

    // Parse qualitative metrics
    const qualMatch = criteriaSection.match(/###\s+Qualitative Metrics([\s\S]*?)(?=\n###|$)/i);
    if (qualMatch) {
      const metrics = qualMatch[1]
        .split(/\n[-*]\s+/)
        .map((m) => m.trim())
        .filter((m) => m.length > 0);
      criteria.qualitative = metrics;
    }

    // Parse security metrics (if present)
    const securityMatch = criteriaSection.match(/###\s+Security Metrics([\s\S]*?)(?=\n###|$)/i);
    if (securityMatch) {
      const metrics = securityMatch[1]
        .split(/\n[-*]\s+/)
        .map((m) => m.trim())
        .filter((m) => m.length > 0);
      criteria.security = metrics;
    }

    // Parse performance metrics (if present)
    const perfMatch = criteriaSection.match(/###\s+Performance Metrics([\s\S]*?)(?=\n###|$)/i);
    if (perfMatch) {
      const metrics = perfMatch[1]
        .split(/\n[-*]\s+/)
        .map((m) => m.trim())
        .filter((m) => m.length > 0);
      criteria.performance = metrics;
    }

    return criteria;
  }

  private extractClarifications(content: string): string[] {
    const clarifications: string[] = [];
    const clarificationRegex = /\[NEEDS CLARIFICATION:\s*([^\]]+)\]/g;
    let match;

    while ((match = clarificationRegex.exec(content)) !== null) {
      clarifications.push(match[1].trim());
    }

    return clarifications;
  }
}
