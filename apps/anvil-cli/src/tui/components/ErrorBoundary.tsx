import React, { Component, type ReactNode } from 'react';
import { Box, Text, useInput, useApp } from 'ink';
import { theme } from '../utils/theme.js';

interface Props {
  children?: ReactNode;
  componentName?: string;
  onRetry?: () => void;
  onExit?: () => void;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: React.ErrorInfo | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null, errorInfo: null };
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  componentDidCatch(_error: Error, errorInfo: React.ErrorInfo): void {
    this.setState({ errorInfo });
  }

  render(): ReactNode {
    if (this.state.hasError) {
      return (
        <ErrorFallback
          error={this.state.error || new Error('Unknown error')}
          componentName={this.props.componentName}
          onRetry={
            this.props.onRetry
              ? () => {
                  this.setState({ hasError: false, error: null, errorInfo: null });
                  this.props.onRetry?.();
                }
              : undefined
          }
          onExit={this.props.onExit}
        />
      );
    }

    return this.props.children;
  }
}

interface ErrorFallbackProps {
  error: Error;
  componentName?: string;
  onRetry?: () => void;
  onExit?: () => void;
}

export function ErrorFallback({
  error,
  componentName = 'component',
  onRetry,
  onExit,
}: ErrorFallbackProps): React.ReactElement {
  const { exit } = useApp();

  useInput((input) => {
    if (input === 'r' && onRetry) {
      onRetry();
    } else if (input === 'q') {
      if (onExit) onExit();
      exit();
    }
  });

  return (
    <Box flexDirection="column" paddingX={2} paddingY={1}>
      <Box marginBottom={1}>
        <Text bold color={theme.colours.slag}>
          {theme.icons.error} Something went wrong
        </Text>
      </Box>

      <Box marginBottom={1}>
        <Text color={theme.colours.ash}>The {componentName} encountered an unexpected error.</Text>
      </Box>

      <Box marginBottom={1} paddingX={1} borderStyle="single" borderColor={theme.colours.charcoal}>
        <Text color={theme.colours.molten}>{error.message}</Text>
      </Box>

      <Box flexDirection="column">
        {onRetry && <Text color={theme.colours.smoke}>Press r to retry</Text>}
        <Text color={theme.colours.smoke}>Press q to exit</Text>
        <Text color={theme.colours.smoke}>
          If this persists, please report: https://github.com/EddaCraft/anvil-001/issues
        </Text>
      </Box>
    </Box>
  );
}
