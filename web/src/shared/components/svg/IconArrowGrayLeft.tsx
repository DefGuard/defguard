import type { SVGProps } from 'react';
const SvgIconArrowGrayLeft = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-arrow-gray-left_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            fill: '#899ca8',
            opacity: 0,
          }}
        />
      </clipPath>
      <style>{'.icon-arrow-gray-left_svg__c{fill:#899ca8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-arrow-gray-left_svg__a)',
      }}
    >
      <rect
        width={8}
        height={2}
        className="icon-arrow-gray-left_svg__c"
        rx={1}
        transform="rotate(-45 16.742 -2.863)"
      />
      <rect
        width={8}
        height={2}
        className="icon-arrow-gray-left_svg__c"
        rx={1}
        transform="rotate(-135 9.814 5.893)"
      />
    </g>
  </svg>
);
export default SvgIconArrowGrayLeft;
