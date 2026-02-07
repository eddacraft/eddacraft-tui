import React from 'react';
import { Box, Text } from 'ink';
import { renderMermaidAscii } from 'beautiful-mermaid';
import type { AsciiRenderOptions } from 'beautiful-mermaid';
import { theme } from '../utils/theme.js';

export interface MermaidDiagramProps {
  /** Mermaid diagram definition (e.g. "graph TD\n  A --> B") */
  definition: string;
  /** Colour for the rendered text (defaults to theme.colours.ash) */
  colour?: string;
  /** Options passed to renderMermaidAscii */
  asciiOptions?: AsciiRenderOptions;
}

export function MermaidDiagram({
  definition,
  colour,
  asciiOptions,
}: MermaidDiagramProps): React.ReactElement {
  const ascii = React.useMemo(() => {
    try {
      return renderMermaidAscii(definition, {
        paddingX: 2,
        paddingY: 1,
        ...asciiOptions,
      });
    } catch {
      // Fall back to raw definition if parsing fails
      return definition;
    }
  }, [definition, asciiOptions]);

  return (
    <Box flexDirection="column">
      {ascii.split('\n').map((line, i) => (
        <Text key={i} color={colour ?? theme.colours.ash}>
          {line}
        </Text>
      ))}
    </Box>
  );
}
