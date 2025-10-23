import { revalidateLogic } from '@tanstack/react-form';

export const formChangeLogic = revalidateLogic({
  mode: 'change',
  modeAfterSubmission: 'change',
});
