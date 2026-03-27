import { useMutation } from "@tanstack/react-query";
import { useMemo } from "react";
import z from "zod";
import { useShallow } from "zustand/react/shallow";
import { m } from "../../../paraglide/messages";
import api from "../../../shared/api/api";
import { Controls } from "../../../shared/components/Controls/Controls";
import { WizardCard } from "../../../shared/components/wizard/WizardCard/WizardCard";
import { Button } from "../../../shared/defguard-ui/components/Button/Button";
import { SizedBox } from "../../../shared/defguard-ui/components/SizedBox/SizedBox";
import { ThemeSpacing } from "../../../shared/defguard-ui/types";
import { useAppForm } from "../../../shared/form";
import { formChangeLogic } from "../../../shared/formLogic";
import { useMigrationWizardStore } from "../store/useMigrationWizardStore";

export const MigrationWizardGeneralConfigurationStep = () => {
	const { mutateAsync } = useMutation({
		mutationFn: api.settings.patchSettings,
		meta: {
			invalidate: [["settings"], ["migration", "state"]],
		},
	});

	const formSchema = useMemo(
		() =>
			z.object({
				default_admin_group_name: z
					.string()
					.min(
						1,
						m.migration_wizard_general_config_error_admin_group_required(),
					),
				authentication_period_days: z
					.number()
					.min(1, m.migration_wizard_general_config_error_auth_period_min()),
				mfa_code_timeout_seconds: z
					.number()
					.min(60, m.migration_wizard_general_config_error_mfa_timeout_min()),
			}),
		[],
	);
	type FormFields = z.infer<typeof formSchema>;

	const defaultValues = useMigrationWizardStore(
		useShallow(
			(s): FormFields => ({
				default_admin_group_name: s.default_admin_group_name,
				authentication_period_days: s.authentication_period_days,
				mfa_code_timeout_seconds: s.mfa_code_timeout_seconds,
			}),
		),
	);

	const form = useAppForm({
		defaultValues,
		validationLogic: formChangeLogic,
		validators: {
			onSubmit: formSchema,
			onChange: formSchema,
		},
		onSubmit: async ({ value }) => {
			await mutateAsync(value);
			useMigrationWizardStore.setState(value);
			useMigrationWizardStore.getState().next();
		},
	});

	return (
		<WizardCard>
			<form
				onSubmit={(e) => {
					e.stopPropagation();
					e.preventDefault();
					form.handleSubmit();
				}}
			>
				<form.AppForm>
					<form.AppField name="default_admin_group_name">
						{(field) => (
							<field.FormInput
								required
								label={m.migration_wizard_general_config_label_admin_group()}
								type="text"
							/>
						)}
					</form.AppField>
					<SizedBox height={ThemeSpacing.Xl} />
					<form.AppField name="authentication_period_days">
						{(field) => (
							<field.FormInput
								required
								label={m.migration_wizard_general_config_label_auth_period()}
								type="number"
							/>
						)}
					</form.AppField>
					<SizedBox height={ThemeSpacing.Xl} />
					<form.AppField name="mfa_code_timeout_seconds">
						{(field) => (
							<field.FormInput
								required
								label={m.migration_wizard_general_config_label_mfa_timeout()}
								type="number"
							/>
						)}
					</form.AppField>
					<form.Subscribe selector={(s) => s.isSubmitting}>
						{(isSubmitting) => (
							<Controls>
								<Button
									variant="outlined"
									text={m.controls_back()}
									disabled={isSubmitting}
									onClick={() => {
										useMigrationWizardStore.getState().back();
									}}
								/>
								<div className="right">
									<Button
										text={m.controls_continue()}
										type="submit"
										loading={isSubmitting}
									/>
								</div>
							</Controls>
						)}
					</form.Subscribe>
				</form.AppForm>
			</form>
		</WizardCard>
	);
};
