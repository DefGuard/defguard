import { useQuery } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { SelectionSection } from '../../../shared/components/SelectionSection/SelectionSection';
import type { SelectionSectionOption } from '../../../shared/components/SelectionSection/type';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

export const AddLocationAccessStep = () => {
  const [selected, setSelected] = useState<Set<string>>(
    new Set(useAddLocationStore.getState().allowed_groups),
  );

  const { data: groups } = useQuery({
    queryFn: api.group.getGroups,
    queryKey: ['group'],
    select: (resp) => resp.data.groups,
  });

  const selectionOptions = useMemo(() => {
    if (!groups) return [];
    return groups.map(
      (group): SelectionSectionOption<string> => ({
        id: group,
        label: group,
      }),
    );
  }, [groups]);

  const saveChanges = useCallback((values: Set<string>) => {
    useAddLocationStore.setState({
      allowed_groups: Array.from(values),
    });
  }, []);

  return (
    <WizardCard>
      <SelectionSection
        options={selectionOptions}
        selection={selected}
        onChange={setSelected}
      />
      <ModalControls
        submitProps={{
          text: m.controls_continue(),
          onClick: () => {
            saveChanges(selected);
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.Firewall,
            });
          },
        }}
      >
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            saveChanges(selected);
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.Mfa,
            });
          }}
        />
      </ModalControls>
    </WizardCard>
  );
};
