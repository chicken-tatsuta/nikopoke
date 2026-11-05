import { createTheme } from '@mui/material/styles';

const ink = '#111111';
const note = '#f5eee4';
export const toolsTheme = createTheme({
  palette: {
    primary: { main: ink }, secondary: { main: '#555555' },
    text: { primary: ink, secondary: '#666666' },
    background: { default: '#ffffff', paper: '#ffffff' }, divider: '#dddddd',
  },
  shape: { borderRadius: 8 },
  typography: {
    fontFamily: '"Hiragino Kaku Gothic ProN", "Yu Gothic", "Noto Sans JP", sans-serif',
    allVariants: { letterSpacing: 0 },
    h4: { fontSize: '1.75rem', fontWeight: 800 },
    h5: { fontSize: '1.5rem', fontWeight: 800 },
    h6: { fontSize: '1.125rem', fontWeight: 700 },
    body1: { fontSize: '0.875rem' }, body2: { fontSize: '0.8125rem' },
    button: { textTransform: 'none', fontWeight: 700 },
  },
  components: {
    MuiContainer: { styleOverrides: { root: { paddingLeft: '0 !important', paddingRight: '0 !important' } } },
    MuiPaper: { styleOverrides: { root: {
      boxShadow: 'none !important', backgroundImage: 'none !important',
      borderRadius: '8px !important',
    } } },
    MuiCard: { styleOverrides: { root: {
      border: `1px solid ${ink}`, background: '#ffffff !important',
    } } },
    MuiCardHeader: { styleOverrides: {
      root: { padding: '16px 20px', borderBottom: `1px solid ${ink}` },
      title: { fontSize: '1rem', fontWeight: 800 },
      subheader: { fontSize: '0.75rem', marginTop: 4 },
    } },
    MuiCardContent: { styleOverrides: { root: { padding: 20 } } },
    MuiButton: { defaultProps: { disableElevation: true }, styleOverrides: {
      root: { borderRadius: 6, minHeight: 36, fontSize: '0.8125rem' },
      outlined: { borderColor: ink },
    } },
    MuiOutlinedInput: { styleOverrides: {
      root: { fontSize: '0.875rem', backgroundColor: '#ffffff', borderRadius: 6 },
      notchedOutline: { borderColor: '#999999' },
    } },
    MuiInputLabel: { styleOverrides: { root: { fontSize: '0.875rem' } } },
    MuiChip: { styleOverrides: { root: {
      borderRadius: '4px !important', backgroundColor: '#ffffff !important',
      color: `${ink} !important`, border: `1px solid ${ink}`, fontWeight: 700,
      fontSize: '0.6875rem',
    }, clickable: { '&:hover': { backgroundColor: `${note} !important` } } } },
    MuiTableCell: { styleOverrides: {
      root: { fontSize: '0.8125rem', borderBottomColor: '#dddddd' },
      head: { backgroundColor: note, color: ink, borderBottom: `1px solid ${ink}`, whiteSpace: 'nowrap' },
    } },
    MuiSlider: { styleOverrides: {
      root: { color: ink }, track: { backgroundColor: `${ink} !important` },
      rail: { backgroundColor: '#dddddd !important' },
      thumb: { border: `2px solid ${ink} !important`, boxShadow: 'none !important' },
    } },
    MuiMenuItem: { styleOverrides: { root: {
      fontSize: '0.875rem', '&.Mui-selected': { backgroundColor: note },
    } } },
    MuiAccordion: { styleOverrides: { root: { boxShadow: 'none' } } },
  },
});
