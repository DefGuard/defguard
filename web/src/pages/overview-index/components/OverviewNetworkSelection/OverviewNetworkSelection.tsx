import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Select } from '../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSizeVariant,
} from '../../../../shared/defguard-ui/components/Layout/Select/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import useApi from '../../../../shared/hooks/useApi';

export const OverviewNetworkSelection = () => {
  const navigate = useNavigate();
  const { networkId } = useParams();
  const { LL } = useI18nContext();
  const localLL = LL.networkOverview;
  const location = useLocation();

  const {
    network: { getNetworks },
  } = useApi();

  const { data: networks, isLoading } = useQuery({
    queryKey: ['network'],
    queryFn: getNetworks,
    placeholderData: (perv) => perv,
  });

  const selectionValue = useMemo(() => {
    if (networkId) {
      const value = parseInt(networkId);
      if (!isNaN(value) && typeof value === 'number') {
        return value;
      }
    }
    return null;
  }, [networkId]);

  const options = useMemo(() => {
    const res: SelectOption<number | null>[] = [
      {
        key: 'all',
        label: localLL.networkSelection.all(),
        value: null,
      },
    ];
    if (networks) {
      for (const network of networks) {
        res.push({
          key: network.id,
          label: network.name,
          value: network.id,
        });
      }
    }
    return res;
  }, [localLL.networkSelection, networks]);

  return (
    <Select
      sizeVariant={SelectSizeVariant.SMALL}
      selected={selectionValue}
      options={options}
      placeholder={localLL.networkSelection.placeholder()}
      loading={isLoading && !isPresent(networks)}
      onChangeSingle={(networkId) => {
        if (networkId !== null) {
          navigate(`/admin/overview/${networkId}${location.search}`);
        } else {
          navigate(`/admin/overview${location.search}`);
        }
      }}
    />
  );
};
