import { createFileRoute } from '@tanstack/react-router';
import { PostureChecksPage } from '../../../../pages/PostureChecksPage/PostureChecksPage';

export const Route = createFileRoute('/_authorized/_default/acl/posture-checks')({
  component: PostureChecksPage,
});
