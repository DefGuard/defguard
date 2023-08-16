import type { SVGProps } from 'react';
const SvgIconCheckmarkWhiteBig = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={36}
    height={36}
    viewBox="0 0 36 36"
    {...props}
  >
    <defs>
      <clipPath id="icon-checkmark-white-big_svg__a">
        <path
          d="M0 0h36v36H0z"
          style={{
            opacity: 0,
            fill: '#fff',
          }}
          transform="translate(.203 .203)"
        />
      </clipPath>
      <style>{'.icon-checkmark-white-big_svg__c{fill:#fff}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-checkmark-white-big_svg__a)',
      }}
      transform="rotate(90 18.203 18)"
    >
      <rect
        width={19.857}
        height={3.31}
        className="icon-checkmark-white-big_svg__c"
        rx={1.655}
        transform="rotate(45 -.06 17.375)"
      />
      <rect
        width={13.238}
        height={3.31}
        className="icon-checkmark-white-big_svg__c"
        rx={1.655}
        transform="rotate(-45 39.078 -4.59)"
      />
    </g>
  </svg>
);
export default SvgIconCheckmarkWhiteBig;
