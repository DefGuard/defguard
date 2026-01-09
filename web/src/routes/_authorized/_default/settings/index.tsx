import { createFileRoute } from '@tanstack/react-router';
import z from 'zod';
import { SettingsIndexPage } from '../../../../pages/settings/SettingsIndexPage/SettingsIndexPage';
import { settingsTabsSchema } from '../../../../pages/settings/SettingsIndexPage/types';

const searchSchema = z.object({
  tab: settingsTabsSchema.optional().default('general'),
});

export const Route = createFileRoute('/_authorized/_default/settings/')({
  validateSearch: searchSchema,
  component: SettingsIndexPage,
});
