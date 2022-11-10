import { ContentCard } from '../../../shared/components/layout/ContentCard/ContentCard';

export const SupportCard = () => {
  return (
    <ContentCard title="Support">
      <p>
        For Community support Please visit:
        <br />
        <a href="https://github.com/Defguard/defguard" className="link">
          github.com/Defguard/core
        </a>
      </p>
      <br />
      <p>
        for Enterprise support
        <br /> Please contact:{' '}
        <a
          className="link"
          onClick={() =>
            (window.location.href = 'mailto:community@defguard.net')
          }
        >
          support@defguard.net
        </a>
      </p>
    </ContentCard>
  );
};
