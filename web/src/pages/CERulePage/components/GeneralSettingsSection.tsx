import type { NetworkLocation } from '../../../shared/api/types';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import type {
  SelectionOption,
  SelectionSectionCustomRender,
} from '../../../shared/components/SelectionSection/type';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { CheckboxIndicator } from '../../../shared/defguard-ui/components/CheckboxIndicator/CheckboxIndicator';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Icon, type IconKindValue } from '../../../shared/defguard-ui/components/Icon';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TooltipContent } from '../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { TextStyle, ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import type { CERuleFormApi, FormFieldRenderer } from '../types';

const renderLocationSelectionItem: SelectionSectionCustomRender<
  number,
  NetworkLocation
> = ({ active, onClick, option }) => {
  const icon: IconKindValue = 'check';
  return (
    <div className="item location-selection-item" onClick={onClick}>
      <CheckboxIndicator active={active} />
      {isPresent(option.meta) && (
        <>
          <div className="content-track">
            <p className="item-label">{option.meta?.name}</p>
          </div>
          <TooltipProvider>
            <TooltipTrigger>
              <Icon icon={icon} size={16} />
            </TooltipTrigger>
            <TooltipContent>
              {!option.meta.acl_enabled && (
                <p>{`Location access unmanaged (ACL disabled)`}</p>
              )}
              {option.meta.acl_enabled && option.meta.acl_default_allow && (
                <p>{`Location access allowed by default - network traffic not explicitly defined by the rules will be passed.`}</p>
              )}
              {option.meta.acl_enabled && !option.meta.acl_default_allow && (
                <p>{`Location access denied by default - network traffic not explicitly defined by the rules will be blocked.`}</p>
              )}
            </TooltipContent>
          </TooltipProvider>
        </>
      )}
    </div>
  );
};

type Props = {
  form: unknown;
  locationsOptions: SelectionOption<number>[];
};

export const GeneralSettingsSection = ({ form, locationsOptions }: Props) => {
  const Form = form as CERuleFormApi;

  return (
    <MarkedSection icon="settings">
      <AppText font={TextStyle.TBodyPrimary600}>{`General settings`}</AppText>
      <SizedBox height={ThemeSpacing.Xl} />
      <Form.AppField name="name">
        {(field) => {
          const Field = field as FormFieldRenderer;
          return <Field.FormInput required label="Rule name" />;
        }}
      </Form.AppField>
      <Divider spacing={ThemeSpacing.Xl2} />
      <DescriptionBlock title="Locations">
        <p>{`Specify which locations this rule applies to. You can select all available locations or choose specific ones based on your requirements.`}</p>
      </DescriptionBlock>
      <SizedBox height={ThemeSpacing.Xl} />
      <Form.Subscribe
        selector={(s) =>
          (s as { values: { all_locations: boolean } }).values.all_locations
        }
      >
        {(allValue) => (
          <Form.AppField name="locations">
            {(field) => {
              const Field = field as FormFieldRenderer;
              return (
                <Field.FormSelectMultiple
                  options={locationsOptions}
                  counterText={(counter: number) => `Locations ${counter}`}
                  editText="Edit locations"
                  modalTitle="Select locations"
                  toggleText="Include all locations"
                  selectionCustomItemRender={renderLocationSelectionItem}
                  toggleValue={allValue}
                  onToggleChange={(value: boolean) => {
                    Form.setFieldValue('all_locations', value);
                  }}
                />
              );
            }}
          </Form.AppField>
        )}
      </Form.Subscribe>
    </MarkedSection>
  );
};
