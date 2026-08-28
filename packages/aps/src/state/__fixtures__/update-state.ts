import { updateTaskState } from '../index.js';

const [projectRoot, taskId] = process.argv.slice(2);
if (!projectRoot || !taskId) {
  throw new Error('usage: update-state.ts <project-root> <task-id>');
}

await updateTaskState(projectRoot, taskId, {
  status: 'locked',
  locked_at: '2025-12-17T10:00:00.000Z',
  locked_by: taskId,
});
