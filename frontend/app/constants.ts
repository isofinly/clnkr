export const MODELS = [
  { value: "gemini-flash", label: "gemini flash" },
  { value: "gemini-flash-lite", label: "gemini flash lite" },
] as const;

export const ASCII_LOGO = `
    .----------.
    |  []  []  |
    |    __    |
    |   (  )   |
    '----||----'
    /----||----\\
   / .--'  '--.\\
  /_/   CLNKR  \\_\\
  |   |      |  |
  |   |      |  |
  |___|      |__|
   \\_/        \\_/
`;

export const ASCII_SPINNER = ["|", "/", "-", "\\"];

export const SPEAKER_COLORS = [
  "#e06c75",
  "#61afef",
  "#98c379",
  "#e5c07b",
  "#c678dd",
  "#56b6c2",
  "#d19a66",
  "#abb2bf",
];


export const API_BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL || "http://localhost:8080";
