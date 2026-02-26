import { flat } from 'radashi';
import type {
  AclAlias,
  AclDestination,
  AclProtocolValue,
} from '../../../shared/api/types';
import { AclProtocolName, aclProtocolValues } from '../../../shared/api/types';
import { Card } from '../../../shared/components/Card/Card';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { DestinationDismissibleBox } from '../../../shared/components/DestinationDismissibleBox/DestinationDismissibleBox';
import { DestinationLabel } from '../../../shared/components/DestinationLabel/DestinationLabel';
import { useSelectionModal } from '../../../shared/components/modals/SelectionModal/useSelectionModal';
import type {
  SelectionOption,
  SelectionSectionCustomRender,
} from '../../../shared/components/SelectionSection/type';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { ButtonsGroup } from '../../../shared/defguard-ui/components/ButtonsGroup/ButtonsGroup';
import { CheckboxIndicator } from '../../../shared/defguard-ui/components/CheckboxIndicator/CheckboxIndicator';
import { Chip } from '../../../shared/defguard-ui/components/Chip/Chip';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import type { CERuleFormApi, FormFieldRenderer } from '../types';

const getProtocolName = (key: AclProtocolValue) => AclProtocolName[key];

const renderDestinationSelectionItem: SelectionSectionCustomRender<
  number,
  AclDestination
> = ({ active, onClick, option }) => (
  <div className="destination-selection-item" onClick={onClick}>
    <CheckboxIndicator active={active} />
    {isPresent(option.meta) && (
      <DestinationLabel
        name={option.meta.name}
        ips={option.meta.addresses}
        ports={option.meta.ports}
        protocols={option.meta.protocols
          .map((protocol) => AclProtocolName[protocol])
          .join(',')}
      />
    )}
  </div>
);

type Props = {
  form: unknown;
  destinations: AclDestination[] | undefined;
  destinationsOptions: SelectionOption<number>[] | undefined;
  aliasesOptions: SelectionOption<number>[] | undefined;
  selectedAliases: AclAlias[];
  aliasesEmptyImage: string;
  emptyDestinationIconSrc: string;
};

export const DestinationSection = ({
  form,
  destinations,
  destinationsOptions,
  aliasesOptions,
  selectedAliases,
  aliasesEmptyImage,
  emptyDestinationIconSrc,
}: Props) => {
  const Form = form as CERuleFormApi;

  return (
    <MarkedSection icon="location-tracking">
      <AppText font={TextStyle.TBodyPrimary600}>{`Destination`}</AppText>
      <SizedBox height={ThemeSpacing.Sm} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {`You can add additional destinations to this rule to extend its scope. These destinations are configured separately in the 'Destinations' section.`}
      </AppText>
      <SizedBox height={ThemeSpacing.Xl2} />
      {isPresent(destinations) && destinations.length === 0 && (
        <div className="no-resource">
          <div className="icon-box">
            <img src={emptyDestinationIconSrc} height={40} width={41} />
          </div>
          <p>{`You don't have any predefined destinations yet — add them in the 'Destinations' section to create reusable elements for defining destinations across multiple firewall ACL rules.`}</p>
        </div>
      )}
      {isPresent(destinations) && destinations.length > 0 && (
        <Form.AppField name="destinations">
          {(field) => {
            const destinationField = field as {
              state: { value: Set<number> };
              handleChange: (value: Set<number>) => void;
            };
            const selectedDestinations =
              destinations?.filter((destination) =>
                destinationField.state.value.has(destination.id),
              ) ?? [];
            return (
              <>
                <Button
                  variant="outlined"
                  text="Select predefined destination(s)"
                  onClick={() => {
                    useSelectionModal.setState({
                      title: 'Select predefined destination(s)',
                      isOpen: true,
                      options: destinationsOptions ?? [],
                      itemGap: 12,
                      enableDividers: true,
                      onSubmit: (selection) =>
                        destinationField.handleChange(new Set(selection as number[])),
                      // @ts-expect-error
                      renderItem: renderDestinationSelectionItem,
                    });
                  }}
                />
                {selectedDestinations.length > 0 && (
                  <div className="selected-destinations">
                    <div className="top">
                      <p>{`Selected destinations`}</p>
                    </div>
                    <div className="items-track">
                      {selectedDestinations.map((destination) => (
                        <DestinationDismissibleBox
                          key={destination.id}
                          name={destination.name}
                          ips={destination.addresses}
                          ports={destination.ports}
                          protocols={destination.protocols
                            .map((p) => AclProtocolName[p])
                            .join(',')}
                          onClick={() => {
                            const newValue = new Set(destinationField.state.value);
                            newValue.delete(destination.id);
                            destinationField.handleChange(newValue);
                          }}
                        />
                      ))}
                    </div>
                  </div>
                )}
              </>
            );
          }}
        </Form.AppField>
      )}
      <Divider text="or/and" spacing={ThemeSpacing.Lg} />
      <DescriptionBlock title={`Define destination manually`}>
        <p>{`Manually configure destinations parameters for this rule.`}</p>
      </DescriptionBlock>
      <SizedBox height={ThemeSpacing.Xl} />
      <Form.AppField name="use_manual_destination_settings">
        {(field) => {
          const Field = field as FormFieldRenderer;
          return <Field.FormCheckbox text="Add manual destination settings" />;
        }}
      </Form.AppField>
      <Form.Subscribe
        selector={(s) =>
          (s as { values: { use_manual_destination_settings: boolean } }).values
            .use_manual_destination_settings
        }
      >
        {(open) => (
          <Fold open={Boolean(open)}>
            <SizedBox height={ThemeSpacing.Xl2} />
            <Card>
              {isPresent(aliasesOptions) && aliasesOptions.length === 0 && (
                <div className="no-resource">
                  <div className="icon-box">
                    <img src={aliasesEmptyImage} height={40} />
                  </div>
                  <p>{`You don't have any aliases to use yet — create them in the “Aliases” section to create reusable elements for defining destinations in multiple firewall ACL rules.`}</p>
                </div>
              )}
              {isPresent(aliasesOptions) && aliasesOptions.length > 0 && (
                <>
                  <DescriptionBlock title="Aliases">
                    <p>{`Aliases can optionally define some or all of the manual destination settings. They are combined with the values you specify to form the final destination for firewall rule generation.`}</p>
                  </DescriptionBlock>
                  <SizedBox height={ThemeSpacing.Lg} />
                  <Form.AppField name="aliases">
                    {(field) => {
                      const aliasesField = field as {
                        state: { value: Set<number> };
                        handleChange: (value: Set<number>) => void;
                      };
                      return (
                        <>
                          <ButtonsGroup>
                            <Button
                              variant="outlined"
                              text="Apply aliases"
                              disabled={aliasesOptions?.length === 0}
                              onClick={() => {
                                useSelectionModal.setState({
                                  isOpen: true,
                                  onSubmit: (selected) => {
                                    aliasesField.handleChange(
                                      new Set(selected as number[]),
                                    );
                                  },
                                  options: aliasesOptions,
                                  selected: new Set(aliasesField.state.value),
                                  title: 'Select Aliases',
                                });
                              }}
                            />
                          </ButtonsGroup>
                          <SizedBox height={ThemeSpacing.Xl} />
                          {isPresent(aliasesOptions) &&
                            aliasesOptions
                              .filter((alias) => aliasesField.state.value.has(alias.id))
                              .map((option) => (
                                <Chip
                                  size="sm"
                                  text={option.label}
                                  key={option.id}
                                  onDismiss={() => {
                                    const newState = new Set(aliasesField.state.value);
                                    newState.delete(option.id);
                                    aliasesField.handleChange(newState);
                                  }}
                                />
                              ))}
                        </>
                      );
                    }}
                  </Form.AppField>
                </>
              )}
              <Divider spacing={ThemeSpacing.Xl} />
              <DescriptionBlock title="Addresses/Ranges">
                <p>{`Define the IP addresses or ranges that form the destination of this ACL rule.`}</p>
              </DescriptionBlock>
              <SizedBox height={ThemeSpacing.Xl} />
              <Form.AppField name="any_address">
                {(field) => {
                  const Field = field as FormFieldRenderer;
                  return <Field.FormToggle label="Any IP Address" />;
                }}
              </Form.AppField>
              <Form.Subscribe
                selector={(s) =>
                  !(s as { values: { any_address: boolean } }).values.any_address
                }
              >
                {(open) => (
                  <Fold open={Boolean(open)}>
                    <SizedBox height={ThemeSpacing.Xl} />
                    <Form.AppField name="addresses">
                      {(field) => {
                        const Field = field as FormFieldRenderer;
                        return (
                          <Field.FormTextarea label="IPv4/IPv6 CIDR ranges or addresses (or multiple values separated by commas)" />
                        );
                      }}
                    </Form.AppField>
                    <AliasDataBlock
                      values={flat(
                        selectedAliases.map((alias) => alias.addresses.split(',')),
                      )}
                    />
                  </Fold>
                )}
              </Form.Subscribe>
              <Divider spacing={ThemeSpacing.Xl} />
              <DescriptionBlock title="Ports">
                <p>{`You may specify the exact ports accessible to users in this location.`}</p>
              </DescriptionBlock>
              <SizedBox height={ThemeSpacing.Xl} />
              <Form.AppField name="any_port">
                {(field) => {
                  const Field = field as FormFieldRenderer;
                  return <Field.FormToggle label="Any port" />;
                }}
              </Form.AppField>
              <Form.Subscribe
                selector={(s) =>
                  !(s as { values: { any_port: boolean } }).values.any_port
                }
              >
                {(open) => (
                  <Fold open={Boolean(open)}>
                    <SizedBox height={ThemeSpacing.Xl} />
                    <Form.AppField name="ports">
                      {(field) => {
                        const Field = field as FormFieldRenderer;
                        return (
                          <Field.FormInput label="Manually defined ports (or multiple values separated by commas)" />
                        );
                      }}
                    </Form.AppField>
                    <AliasDataBlock
                      values={flat(
                        selectedAliases.map((alias) => alias.ports.split(',')),
                      )}
                    />
                  </Fold>
                )}
              </Form.Subscribe>
              <Divider spacing={ThemeSpacing.Xl} />
              <DescriptionBlock title="Protocols">
                <p>{`By default, all protocols are allowed for this location. You can change this configuration, but at least one protocol must remain selected.`}</p>
              </DescriptionBlock>
              <SizedBox height={ThemeSpacing.Xl} />
              <Form.AppField name="any_protocol">
                {(field) => {
                  const Field = field as FormFieldRenderer;
                  return <Field.FormToggle label="Any protocol" />;
                }}
              </Form.AppField>
              <Form.Subscribe
                selector={(s) =>
                  !(s as { values: { any_protocol: boolean } }).values.any_protocol
                }
              >
                {(open) => (
                  <Fold open={Boolean(open)}>
                    <SizedBox height={ThemeSpacing.Xl2} />
                    <Form.AppField name="protocols">
                      {(field) => {
                        const Field = field as FormFieldRenderer;
                        return (
                          <Field.FormCheckboxGroup
                            values={aclProtocolValues}
                            getLabel={getProtocolName}
                          />
                        );
                      }}
                    </Form.AppField>
                    <AliasDataBlock
                      values={flat(
                        selectedAliases.map((alias) =>
                          alias.protocols.map((protocol) => AclProtocolName[protocol]),
                        ),
                      )}
                    />
                  </Fold>
                )}
              </Form.Subscribe>
            </Card>
          </Fold>
        )}
      </Form.Subscribe>
    </MarkedSection>
  );
};

type AliasDataBlockProps = {
  values: string[];
};

const AliasDataBlock = ({ values }: AliasDataBlockProps) => {
  if (values.length === 0) return null;
  return (
    <div className="alias-data-block">
      <div className="top">
        <p>{`Data from aliases`}</p>
      </div>
      <div className="content-track">
        {values.map((value) => (
          <Chip key={value} text={value} />
        ))}
        {values.length > 4 && (
          <button
            onClick={() => {
              openModal(ModalName.DisplayList, {
                title: 'Data from aliases',
                data: values,
              });
            }}
          >
            <span>{`Show all`}</span>
          </button>
        )}
      </div>
    </div>
  );
};
