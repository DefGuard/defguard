import type { SVGProps } from 'react';
const SvgIconArrowWhiteLeft = (props: SVGProps<SVGSVGElement>) => (
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
            fill: '#fff',
          }}
        />
      </clipPath>
      <style>{'.c{fill:#fff}'}</style>
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
        transform="rotate(-45 16.742 -2.863)"
      />
      <rect
        width={8}
        height={2}
        className="c"
        rx={1}
        transform="rotate(-135 9.814 5.893)"
      />
    </g>
  </svg>
);
export default SvgIconArrowWhiteLeft;
