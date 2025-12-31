import React, { useState, useCallback } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import type { Template, TemplateMetadata } from '../../../services/template-loader.js';
import { theme } from '../../utils/theme.js';

export interface TemplateBrowserResult {
  templateId: string;
  variables: Record<string, string>;
}

interface TemplateBrowserProps {
  templates: Template[];
  onSelect: (result: TemplateBrowserResult) => void;
  onCancel: () => void;
}

type ViewState = 'categories' | 'templates' | 'variables';

export function TemplateBrowser({
  templates,
  onSelect,
  onCancel,
}: TemplateBrowserProps): React.ReactElement {
  const { exit } = useApp();
  const [view, setView] = useState<ViewState>('categories');
  const [selectedCategory, setSelectedCategory] = useState<TemplateMetadata['category'] | null>(
    null
  );
  const [selectedTemplate, setSelectedTemplate] = useState<Template | null>(null);
  const [categoryIndex, setCategoryIndex] = useState(0);
  const [templateIndex, setTemplateIndex] = useState(0);
  const [variableIndex, setVariableIndex] = useState(0);
  const [variables, setVariables] = useState<Record<string, string>>({});
  const [inputValue, setInputValue] = useState('');

  const categories = React.useMemo(() => {
    const cats = new Set<TemplateMetadata['category']>();
    templates.forEach((t) => cats.add(t.metadata.category));
    return Array.from(cats);
  }, [templates]);

  const categoryTemplates = React.useMemo(() => {
    if (!selectedCategory) return [];
    return templates.filter((t) => t.metadata.category === selectedCategory);
  }, [templates, selectedCategory]);

  const requiredVariables = React.useMemo(() => {
    if (!selectedTemplate) return [];
    return selectedTemplate.metadata.variables.filter((v) => v.required && !v.default);
  }, [selectedTemplate]);

  const handleCategorySelect = useCallback(() => {
    setSelectedCategory(categories[categoryIndex]);
    setTemplateIndex(0);
    setView('templates');
  }, [categories, categoryIndex]);

  const handleTemplateSelect = useCallback(() => {
    const template = categoryTemplates[templateIndex];
    setSelectedTemplate(template);

    const requiredVars = template.metadata.variables.filter((v) => v.required && !v.default);
    if (requiredVars.length === 0) {
      onSelect({ templateId: template.metadata.id, variables: {} });
      exit();
    } else {
      setVariableIndex(0);
      setInputValue(requiredVars[0]?.default || '');
      setView('variables');
    }
  }, [categoryTemplates, templateIndex, onSelect, exit]);

  const handleVariableSubmit = useCallback(() => {
    const currentVar = requiredVariables[variableIndex];
    if (!currentVar) return;

    const newVariables = { ...variables, [currentVar.name]: inputValue };
    setVariables(newVariables);

    if (variableIndex < requiredVariables.length - 1) {
      setVariableIndex(variableIndex + 1);
      setInputValue(requiredVariables[variableIndex + 1]?.default || '');
    } else {
      onSelect({
        templateId: selectedTemplate!.metadata.id,
        variables: newVariables,
      });
      exit();
    }
  }, [requiredVariables, variableIndex, inputValue, variables, selectedTemplate, onSelect, exit]);

  const handleBack = useCallback(() => {
    if (view === 'templates') {
      setView('categories');
      setSelectedCategory(null);
    } else if (view === 'variables') {
      setView('templates');
      setSelectedTemplate(null);
      setVariables({});
    }
  }, [view]);

  useInput((input, key) => {
    if (view === 'variables') {
      if (key.return) {
        handleVariableSubmit();
      } else if (key.escape) {
        handleBack();
      } else if (key.backspace || key.delete) {
        setInputValue((prev) => prev.slice(0, -1));
      } else if (input && !key.ctrl && !key.meta) {
        setInputValue((prev) => prev + input);
      }
      return;
    }

    if (key.escape) {
      if (view === 'categories') {
        onCancel();
        exit();
      } else {
        handleBack();
      }
      return;
    }

    if (key.return) {
      if (view === 'categories') {
        handleCategorySelect();
      } else if (view === 'templates') {
        handleTemplateSelect();
      }
      return;
    }

    if (input === 'k' || key.upArrow) {
      if (view === 'categories') {
        setCategoryIndex((prev) => Math.max(0, prev - 1));
      } else if (view === 'templates') {
        setTemplateIndex((prev) => Math.max(0, prev - 1));
      }
    }

    if (input === 'j' || key.downArrow) {
      if (view === 'categories') {
        setCategoryIndex((prev) => Math.min(categories.length - 1, prev + 1));
      } else if (view === 'templates') {
        setTemplateIndex((prev) => Math.min(categoryTemplates.length - 1, prev + 1));
      }
    }
  });

  if (view === 'categories') {
    return (
      <Box flexDirection="column">
        <Box marginBottom={1}>
          <Text bold>Select a category:</Text>
        </Box>
        {categories.map((cat, idx) => (
          <Box key={cat}>
            <Text color={idx === categoryIndex ? 'cyan' : undefined}>
              {idx === categoryIndex ? `${theme.icons.arrow} ` : '  '}
              {cat.toUpperCase()}
            </Text>
          </Box>
        ))}
        <Box marginTop={1}>
          <Text dimColor>Use arrow keys to navigate, Enter to select, Esc to cancel</Text>
        </Box>
      </Box>
    );
  }

  if (view === 'templates') {
    return (
      <Box flexDirection="column">
        <Box marginBottom={1}>
          <Text bold>{selectedCategory?.toUpperCase()} - Select a template:</Text>
        </Box>
        {categoryTemplates.map((template, idx) => (
          <Box key={template.metadata.id} flexDirection="column">
            <Text color={idx === templateIndex ? 'cyan' : undefined}>
              {idx === templateIndex ? `${theme.icons.arrow} ` : '  '}
              {template.metadata.name}
            </Text>
            {idx === templateIndex && (
              <Box marginLeft={4}>
                <Text dimColor>{template.metadata.description}</Text>
              </Box>
            )}
          </Box>
        ))}
        <Box marginTop={1}>
          <Text dimColor>Use arrow keys to navigate, Enter to select, Esc to go back</Text>
        </Box>
      </Box>
    );
  }

  const currentVar = requiredVariables[variableIndex];
  return (
    <Box flexDirection="column">
      <Box marginBottom={1}>
        <Text bold>
          {selectedTemplate?.metadata.name} - Set variables ({variableIndex + 1}/
          {requiredVariables.length}):
        </Text>
      </Box>
      <Box>
        <Text>{currentVar?.name}: </Text>
        <Text color="cyan">{inputValue}</Text>
        <Text color="gray">|</Text>
      </Box>
      <Box marginTop={1}>
        <Text dimColor>{currentVar?.description}</Text>
      </Box>
      <Box marginTop={1}>
        <Text dimColor>Type value and press Enter, Esc to go back</Text>
      </Box>
    </Box>
  );
}
