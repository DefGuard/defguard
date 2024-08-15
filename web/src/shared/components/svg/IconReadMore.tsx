import type { SVGProps } from 'react';
const SvgIconReadMore = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-read-more_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            fill: '#0c8ce0',
            opacity: 0,
          }}
        />
      </clipPath>
      <style>{'.icon-read-more_svg__c{fill:#0c8ce0}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-read-more_svg__a)',
      }}
      transform="translate(-312 -227)"
    >
      <rect
        width={14}
        height={2}
        className="icon-read-more_svg__c"
        rx={1}
        transform="translate(316 233)"
      />
      <rect
        width={10}
        height={2}
        className="icon-read-more_svg__c"
        rx={1}
        transform="translate(316 237)"
      />
      <rect
        width={10}
        height={2}
        className="icon-read-more_svg__c"
        rx={1}
        transform="translate(316 241)"
      />
    </g>
  </svg>
);
export default SvgIconReadMore;
