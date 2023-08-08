import type { SVGProps } from 'react';
const SvgIconArrowGrayRight = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#899ca8',
          }}
        />
      </clipPath>
      <style>{'.c{fill:#899ca8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
    >
      <rect
        width={8}
        height={2}
        className="c"
        rx={1}
        transform="rotate(-135 9.4 3.379)"
      />
      <rect
        width={8}
        height={2}
        className="c"
        rx={1}
        transform="rotate(-45 22.814 -1.864)"
      />
    </g>
  </svg>
);
export default SvgIconArrowGrayRight;
