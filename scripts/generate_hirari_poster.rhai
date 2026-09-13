// ═══════════════════════════════════════════════════════════════════
// HIRARI SOUND 2026 — Official Festival Poster Generator
// Pure Amata Vector Script Engine (A2 Vertical: 1191 × 1684 pt)
// ═══════════════════════════════════════════════════════════════════

let W = 1191.0;
let H = 1684.0;

// ───────────────────────────────────────────────────────────────────
// LAYER 1: Atmosphere & Cosmic Space
// ───────────────────────────────────────────────────────────────────
let bg_objects = [];

// 1. Deep space vertical gradient backdrop
bg_objects.push(#{
    type: "rect",
    name: "Cosmic Backdrop",
    x: 0.0,
    y: 0.0,
    w: W,
    h: H,
    fill: #{
        type: "linear",
        x1: 0.0,
        y1: 0.0,
        x2: 0.0,
        y2: 1.0,
        stops: [
            #{ offset: 0.0, color: "#06040F" },
            #{ offset: 0.35, color: "#0E0824" },
            #{ offset: 0.65, color: "#160A36" },
            #{ offset: 1.0, color: "#070312" }
        ]
    }
});

// 2. Ambient chromatic glowing energy orbs (Screen blend mode)
bg_objects.push(#{
    type: "circle",
    name: "Cyan Energy Orb",
    x: 940.0,
    y: 380.0,
    r: 360.0,
    fill: "#00F5FF",
    opacity: 0.16,
    blend_mode: "Screen"
});

bg_objects.push(#{
    type: "circle",
    name: "Magenta Pulse Orb",
    x: 200.0,
    y: 680.0,
    r: 390.0,
    fill: "#FF007F",
    opacity: 0.15,
    blend_mode: "Screen"
});

bg_objects.push(#{
    type: "circle",
    name: "Solar Gold Core",
    x: 595.5,
    y: 720.0,
    r: 310.0,
    fill: "#FF8800",
    opacity: 0.14,
    blend_mode: "Screen"
});

bg_objects.push(#{
    type: "circle",
    name: "Ultra Violet Basin",
    x: 620.0,
    y: 1350.0,
    r: 450.0,
    fill: "#7928CA",
    opacity: 0.18,
    blend_mode: "Screen"
});

// 3. Technical background grid lines (subtle acoustic studio feel)
let grid_y = 120.0;
while grid_y < 1600.0 {
    bg_objects.push(#{
        type: "line",
        name: "Acoustic Grid H",
        x: 45.0,
        y: grid_y,
        x2: 1146.0,
        y2: grid_y,
        stroke: "#362B55",
        stroke_width: 0.5,
        opacity: 0.25
    });
    grid_y += 180.0;
}

let grid_x = 100.0;
while grid_x < 1150.0 {
    bg_objects.push(#{
        type: "line",
        name: "Acoustic Grid V",
        x: grid_x,
        y: 60.0,
        x2: grid_x,
        y2: 1624.0,
        stroke: "#362B55",
        stroke_width: 0.5,
        opacity: 0.2
    });
    grid_x += 245.0;
}

// 4. Precision outer poster framing border
bg_objects.push(#{
    type: "rect",
    name: "Technical Margin Border",
    x: 40.0,
    y: 40.0,
    w: 1111.0,
    h: 1604.0,
    stroke: "#55437E",
    stroke_width: 1.0,
    fill: "none",
    opacity: 0.4
});

// Corner crosshairs / tick marks
bg_objects.push(#{ type: "line", name: "Tick TL H", x: 30.0, y: 40.0, x2: 50.0, y2: 40.0, stroke: "#00F5FF", stroke_width: 1.5 });
bg_objects.push(#{ type: "line", name: "Tick TL V", x: 40.0, y: 30.0, x2: 40.0, y2: 50.0, stroke: "#00F5FF", stroke_width: 1.5 });
bg_objects.push(#{ type: "line", name: "Tick TR H", x: 1141.0, y: 40.0, x2: 1161.0, y2: 40.0, stroke: "#00F5FF", stroke_width: 1.5 });
bg_objects.push(#{ type: "line", name: "Tick TR V", x: 1151.0, y: 30.0, x2: 1151.0, y2: 50.0, stroke: "#00F5FF", stroke_width: 1.5 });
bg_objects.push(#{ type: "line", name: "Tick BL H", x: 30.0, y: 1644.0, x2: 50.0, y2: 1644.0, stroke: "#00F5FF", stroke_width: 1.5 });
bg_objects.push(#{ type: "line", name: "Tick BL V", x: 40.0, y: 1634.0, x2: 40.0, y2: 1654.0, stroke: "#00F5FF", stroke_width: 1.5 });
bg_objects.push(#{ type: "line", name: "Tick BR H", x: 1141.0, y: 1644.0, x2: 1161.0, y2: 1644.0, stroke: "#00F5FF", stroke_width: 1.5 });
bg_objects.push(#{ type: "line", name: "Tick BR V", x: 1151.0, y: 1634.0, x2: 1151.0, y2: 1654.0, stroke: "#00F5FF", stroke_width: 1.5 });

// ───────────────────────────────────────────────────────────────────
// LAYER 2: Generative Waveforms & Bézier Ribbons
// ───────────────────────────────────────────────────────────────────
let waves_objects = [];

// Concentric sonic dispersion ripples radiating from core (595.5, 710)
let rad = 90.0;
while rad <= 560.0 {
    waves_objects.push(#{
        type: "circle",
        name: "Sonic Ring Ripple",
        x: 595.5,
        y: 710.0,
        r: rad,
        stroke: "#8E72DE",
        stroke_width: 1.0,
        fill: "none",
        opacity: 0.16
    });
    rad += 75.0;
}

// Flowing continuous Bézier wave 1 (Electric Cyan Synthesizer Lead)
waves_objects.push(#{
    type: "path",
    name: "Bézier Synth Wave Cyan",
    stroke: "#00F5FF",
    stroke_width: 3.5,
    fill: "none",
    opacity: 0.85,
    blend_mode: "Screen",
    glow: #{ radius: 14.0, color: "#00F5FF", intensity: 0.7 },
    elements: [
        #{ type: "move_to", x: 40.0, y: 550.0 },
        #{ type: "curve_to", c1_x: 200.0, c1_y: 380.0, c2_x: 380.0, c2_y: 690.0, x: 580.0, y: 520.0 },
        #{ type: "curve_to", c1_x: 750.0, c1_y: 370.0, c2_x: 950.0, c2_y: 740.0, x: 1151.0, y: 480.0 }
    ]
});

// Flowing continuous Bézier wave 2 (Neon Magenta Harmonic Resonance)
waves_objects.push(#{
    type: "path",
    name: "Bézier Synth Wave Magenta",
    stroke: "#FF007F",
    stroke_width: 3.0,
    fill: "none",
    opacity: 0.80,
    blend_mode: "Screen",
    glow: #{ radius: 12.0, color: "#FF007F", intensity: 0.6 },
    elements: [
        #{ type: "move_to", x: 40.0, y: 640.0 },
        #{ type: "curve_to", c1_x: 240.0, c1_y: 820.0, c2_x: 420.0, c2_y: 450.0, x: 620.0, y: 680.0 },
        #{ type: "curve_to", c1_x: 800.0, c1_y: 880.0, c2_x: 980.0, c2_y: 510.0, x: 1151.0, y: 620.0 }
    ]
});

// Flowing continuous Bézier wave 3 (Amber Sub-bass Fluidity)
waves_objects.push(#{
    type: "path",
    name: "Bézier Synth Wave Gold",
    stroke: "#FF9E00",
    stroke_width: 2.5,
    fill: "none",
    opacity: 0.75,
    blend_mode: "Screen",
    elements: [
        #{ type: "move_to", x: 40.0, y: 720.0 },
        #{ type: "curve_to", c1_x: 180.0, c1_y: 560.0, c2_x: 360.0, c2_y: 840.0, x: 570.0, y: 690.0 },
        #{ type: "curve_to", c1_x: 740.0, c1_y: 560.0, c2_x: 930.0, c2_y: 890.0, x: 1151.0, y: 750.0 }
    ]
});

// Translucent filled Bézier undulating ribbon (Cyan -> Magenta gradient)
waves_objects.push(#{
    type: "path",
    name: "Acoustic Fluid Ribbon",
    fill: #{
        type: "linear",
        x1: 0.0,
        y1: 0.0,
        x2: 1.0,
        y2: 0.0,
        stops: [
            #{ offset: 0.0, color: "#00F5FF" },
            #{ offset: 0.5, color: "#7928CA" },
            #{ offset: 1.0, color: "#FF007F" }
        ]
    },
    opacity: 0.22,
    blend_mode: "Screen",
    elements: [
        #{ type: "move_to", x: 40.0, y: 580.0 },
        #{ type: "curve_to", c1_x: 260.0, c1_y: 430.0, c2_x: 440.0, c2_y: 720.0, x: 640.0, y: 560.0 },
        #{ type: "curve_to", c1_x: 820.0, c1_y: 420.0, c2_x: 990.0, c2_y: 750.0, x: 1151.0, y: 540.0 },
        #{ type: "line_to", x: 1151.0, y: 660.0 },
        #{ type: "curve_to", c1_x: 960.0, c1_y: 880.0, c2_x: 780.0, c2_y: 540.0, x: 590.0, y: 700.0 },
        #{ type: "curve_to", c1_x: 410.0, c1_y: 840.0, c2_x: 220.0, c2_y: 540.0, x: 40.0, y: 710.0 },
        #{ type: "close" }
    ]
});

// Second complementary ribbon (Purple -> Orange)
waves_objects.push(#{
    type: "path",
    name: "Secondary Resonance Ribbon",
    fill: #{
        type: "linear",
        x1: 0.0,
        y1: 0.0,
        x2: 1.0,
        y2: 1.0,
        stops: [
            #{ offset: 0.0, color: "#7928CA" },
            #{ offset: 0.6, color: "#FF007A" },
            #{ offset: 1.0, color: "#FF8800" }
        ]
    },
    opacity: 0.18,
    blend_mode: "Screen",
    elements: [
        #{ type: "move_to", x: 40.0, y: 740.0 },
        #{ type: "curve_to", c1_x: 300.0, c1_y: 890.0, c2_x: 520.0, c2_y: 620.0, x: 720.0, y: 780.0 },
        #{ type: "curve_to", c1_x: 900.0, c1_y: 910.0, c2_x: 1050.0, c2_y: 720.0, x: 1151.0, y: 810.0 },
        #{ type: "line_to", x: 1151.0, y: 880.0 },
        #{ type: "curve_to", c1_x: 1000.0, c1_y: 810.0, c2_x: 840.0, c2_y: 990.0, x: 670.0, y: 870.0 },
        #{ type: "curve_to", c1_x: 470.0, c1_y: 740.0, c2_x: 250.0, c2_y: 980.0, x: 40.0, y: 830.0 },
        #{ type: "close" }
    ]
});

// ───────────────────────────────────────────────────────────────────
// LAYER 3: Masked Sonic Core & Clipping Emblem
// ───────────────────────────────────────────────────────────────────
let emblem_objects = [];

// Diamond portal frame outer glow & strokes
let cx = 595.5;
let cy = 710.0;
let dw = 250.0;
let dh = 310.0;

// Outer decorative diamond halo
emblem_objects.push(#{
    type: "path",
    name: "Outer Diamond Halo",
    stroke: "#7928CA",
    stroke_width: 1.5,
    fill: "none",
    opacity: 0.5,
    elements: [
        #{ type: "move_to", x: cx, y: cy - dh - 24.0 },
        #{ type: "line_to", x: cx + dw + 24.0, y: cy },
        #{ type: "line_to", x: cx, y: cy + dh + 24.0 },
        #{ type: "line_to", x: cx - dw - 24.0, y: cy },
        #{ type: "close" }
    ]
});

// Secondary diamond rim with electric cyan corners
emblem_objects.push(#{
    type: "path",
    name: "Aperture Rim Diamond",
    stroke: "#00F5FF",
    stroke_width: 2.5,
    fill: "none",
    opacity: 0.85,
    glow: #{ radius: 18.0, color: "#00F5FF", intensity: 0.8 },
    elements: [
        #{ type: "move_to", x: cx, y: cy - dh },
        #{ type: "line_to", x: cx + dw, y: cy },
        #{ type: "line_to", x: cx, y: cy + dh },
        #{ type: "line_to", x: cx - dw, y: cy },
        #{ type: "close" }
    ]
});

// ── CLIPPING MASK SECTION ──────────────────────────────────────────
// Mask element is the first child; subsequent children are clipped inside.
let mask_children = [];

// Mask Boundary Diamond
mask_children.push(#{
    type: "path",
    name: "Mask Boundary Diamond",
    elements: [
        #{ type: "move_to", x: cx, y: cy - dh + 2.0 },
        #{ type: "line_to", x: cx + dw - 2.0, y: cy },
        #{ type: "line_to", x: cx, y: cy + dh - 2.0 },
        #{ type: "line_to", x: cx - dw + 2.0, y: cy },
        #{ type: "close" }
    ]
});

// Inside Mask: Dark chamber background
mask_children.push(#{
    type: "rect",
    name: "Mask Chamber BG",
    x: cx - dw - 10.0,
    y: cy - dh - 10.0,
    w: dw * 2.0 + 20.0,
    h: dh * 2.0 + 20.0,
    fill: "#0B0418"
});

// Inside Mask: Equalizer spectrum frequency bars
let bar_x = cx - dw + 16.0;
let bar_idx = 0;
while bar_x < cx + dw - 10.0 {
    let t_norm = (bar_x - (cx - dw)) / (dw * 2.0);
    // Envelope height curved to peak near center
    let env = sin(t_norm * pi());
    let bar_h = 40.0 + env * 420.0 * (0.4 + 0.6 * sin((bar_idx * 0.45) + 1.2));
    let bar_y = cy - (bar_h * 0.5);

    let bar_col = if t_norm < 0.33 {
        "#00F5FF"
    } else if t_norm < 0.66 {
        "#FF007F"
    } else {
        "#FFB800"
    };

    mask_children.push(#{
        type: "rect",
        name: "Equalizer Bar",
        x: bar_x,
        y: bar_y,
        w: 6.5,
        h: bar_h,
        corner_radius: 2.0,
        fill: bar_col,
        opacity: 0.85
    });

    bar_x += 12.0;
    bar_idx += 1;
}

// Inside Mask: Central pulsating reactor core
mask_children.push(#{
    type: "circle",
    name: "Core Sound Portal",
    x: cx,
    y: cy,
    r: 95.0,
    fill: #{
        type: "linear",
        x1: 0.0,
        y1: 0.0,
        x2: 1.0,
        y2: 1.0,
        stops: [
            #{ offset: 0.0, color: "#FFFFFF" },
            #{ offset: 0.4, color: "#00F5FF" },
            #{ offset: 1.0, color: "#7928CA" }
        ]
    },
    opacity: 0.9,
    blend_mode: "Screen"
});

// Assemble the Clipping Mask Object
emblem_objects.push(#{
    type: "clipping_mask",
    name: "Sonic Aperture Mask",
    children: mask_children
});

// Central emblem overlay: Stylized vector "A" / Soundwave apex
emblem_objects.push(#{
    type: "path",
    name: "Emblem Central Chevron",
    stroke: "#FFFFFF",
    stroke_width: 4.0,
    fill: "none",
    elements: [
        #{ type: "move_to", x: cx - 45.0, y: cy + 45.0 },
        #{ type: "line_to", x: cx, y: cy - 45.0 },
        #{ type: "line_to", x: cx + 45.0, y: cy + 45.0 }
    ]
});

// ───────────────────────────────────────────────────────────────────
// LAYER 4: Typography & Festival Information
// ─────────────────────────────────────────────────────────────────
let typo_objects = [];

// Category top header
typo_objects.push(#{
    type: "text",
    name: "Header Category",
    text: "INTERNATIONAL AVANT-GARDE AUDIO FESTIVAL  /  EDITION 08",
    x: 80.0,
    y: 95.0,
    size: 15.0,
    fill: "#A894D8"
});

typo_objects.push(#{
    type: "text",
    name: "Header Subtitle",
    text: "SPATIAL SYNTHESIS  •  ACOUSTIC FLUIDITY  •  LIGHT ARCHITECTURE",
    x: 80.0,
    y: 122.0,
    size: 12.5,
    fill: "#00F5FF",
    opacity: 0.9
});

// Top accent divider
typo_objects.push(#{
    type: "line",
    name: "Header Divider",
    x: 80.0,
    y: 140.0,
    x2: 1111.0,
    y2: 140.0,
    stroke: "#4E3C75",
    stroke_width: 1.0,
    opacity: 0.6
});

// MAIN FESTIVAL TITLE: "HIRARI SOUND"
typo_objects.push(#{
    type: "text",
    name: "Title HIRARI SOUND",
    text: "HIRARI SOUND",
    x: 80.0,
    y: 245.0,
    size: 96.0,
    fill: "#FFFFFF",
    shadow: #{ offset_x: 0.0, offset_y: 12.0, blur: 28.0, color: "#7928CA", opacity: 0.85 }
});

// YEAR: "2026"
typo_objects.push(#{
    type: "text",
    name: "Year 2026",
    text: "2 0 2 6",
    x: 80.0,
    y: 335.0,
    size: 68.0,
    fill: "#00F5FF",
    glow: #{ radius: 24.0, color: "#00F5FF", intensity: 0.95 }
});

// Tagline Manifesto
typo_objects.push(#{
    type: "text",
    name: "Tagline Manifesto",
    text: "TRANSCENDING REALITIES THROUGH FREQUENCY MODULATION & OPTICAL KINETICS",
    x: 80.0,
    y: 375.0,
    size: 14.0,
    fill: "#E4DCFA"
});

// Mid-section Festival Date & Venue Box
typo_objects.push(#{
    type: "rect",
    name: "Date & Venue Container",
    x: 80.0,
    y: 1040.0,
    w: 1031.0,
    h: 88.0,
    corner_radius: 8.0,
    fill: "#110926",
    stroke: "#7928CA",
    stroke_width: 1.5,
    opacity: 0.92
});

typo_objects.push(#{
    type: "text",
    name: "Date Badge",
    text: "OCTOBER 17 — 18, 2026",
    x: 105.0,
    y: 1080.0,
    size: 28.0,
    fill: "#FF007F"
});

typo_objects.push(#{
    type: "text",
    name: "Venue Badge",
    text: "MT. FUJI SOUND PARK  •  OPEN AIR AMPHITHEATRE & 64CH SURROUND DOMES",
    x: 105.0,
    y: 1112.0,
    size: 14.5,
    fill: "#FFFFFF"
});

// LINEUP SECTION
typo_objects.push(#{
    type: "text",
    name: "Headliners Label",
    text: "— HEADLINERS —",
    x: 80.0,
    y: 1170.0,
    size: 14.0,
    fill: "#00F5FF"
});

typo_objects.push(#{
    type: "text",
    name: "Headliners Row 1",
    text: "AURAL SPHERE    •    NEON CHRONICLES",
    x: 80.0,
    y: 1215.0,
    size: 34.0,
    fill: "#FFFFFF"
});

typo_objects.push(#{
    type: "text",
    name: "Headliners Row 2",
    text: "VOID SYMPHONY    •    KOSMO PULSE",
    x: 80.0,
    y: 1260.0,
    size: 34.0,
    fill: "#FFFFFF"
});

typo_objects.push(#{
    type: "line",
    name: "Lineup Divider",
    x: 80.0,
    y: 1290.0,
    x2: 1111.0,
    y2: 1290.0,
    stroke: "#4E3C75",
    stroke_width: 1.0,
    opacity: 0.5
});

typo_objects.push(#{
    type: "text",
    name: "Special Guests Label",
    text: "SPECIAL GUESTS & AUDIO-VISUAL ARTISTS",
    x: 80.0,
    y: 1320.0,
    size: 13.0,
    fill: "#FFB800"
});

typo_objects.push(#{
    type: "text",
    name: "Special Guests Row 1",
    text: "RESONANCE COLLECTIVE   •   CYBERNETIC ECHO   •   LUMINA TIDE",
    x: 80.0,
    y: 1355.0,
    size: 21.0,
    fill: "#E4DCFA"
});

typo_objects.push(#{
    type: "text",
    name: "Special Guests Row 2",
    text: "SOLITUDE DUB   •   DRIFT FREQUENCY   •   METEOR SYNTH",
    x: 80.0,
    y: 1390.0,
    size: 21.0,
    fill: "#E4DCFA"
});

typo_objects.push(#{
    type: "text",
    name: "Supporting Artists",
    text: "CHROMA WAVE  /  SUBATOMIC ORCHESTRA  /  NEURAL RESONANCE  /  HYPERSPACE ENSEMBLE  /  KINETIC PHONON",
    x: 80.0,
    y: 1435.0,
    size: 14.5,
    fill: "#9E8EC7"
});

// FOOTER SECTION
typo_objects.push(#{
    type: "line",
    name: "Footer Divider",
    x: 80.0,
    y: 1475.0,
    x2: 1111.0,
    y2: 1475.0,
    stroke: "#4E3C75",
    stroke_width: 1.0,
    opacity: 0.5
});

typo_objects.push(#{
    type: "text",
    name: "Stages Info",
    text: "STAGES:  MAIN MATRIX (TRANSCENDENCE)   |   DOME A (3D SPATIAL LAB)   |   DOME B (MODULAR SYNTH)",
    x: 80.0,
    y: 1510.0,
    size: 12.0,
    fill: "#7E6E9E"
});

typo_objects.push(#{
    type: "text",
    name: "Tickets & Web",
    text: "TICKETS & CAMPING PASSES ON SALE NOW: WWW.HIRARISOUND.JP",
    x: 80.0,
    y: 1545.0,
    size: 15.0,
    fill: "#FFFFFF"
});

typo_objects.push(#{
    type: "text",
    name: "Tech Specifications",
    text: "A2 FORMAT (420 × 594 MM)  •  32-BIT 96KHZ SYSTEM  •  PRODUCED WITH AMATA VECTOR STUDIO",
    x: 80.0,
    y: 1580.0,
    size: 11.0,
    fill: "#00F5FF",
    opacity: 0.85
});

// Decorative vector QR Code simulation in bottom-right corner
let qrx = 1010.0;
let qry = 1495.0;
let qrs = 100.0;
typo_objects.push(#{
    type: "rect",
    name: "QR Code Background",
    x: qrx,
    y: qry,
    w: qrs,
    h: qrs,
    fill: "#FFFFFF",
    corner_radius: 6.0
});

// QR framing markers
typo_objects.push(#{ type: "rect", name: "QR Corner TL", x: qrx + 10.0, y: qry + 10.0, w: 24.0, h: 24.0, fill: "#0A0716" });
typo_objects.push(#{ type: "rect", name: "QR Core TL", x: qrx + 16.0, y: qry + 16.0, w: 12.0, h: 12.0, fill: "#FFFFFF" });
typo_objects.push(#{ type: "rect", name: "QR Dot TL", x: qrx + 19.0, y: qry + 19.0, w: 6.0, h: 6.0, fill: "#7928CA" });

typo_objects.push(#{ type: "rect", name: "QR Corner TR", x: qrx + qrs - 34.0, y: qry + 10.0, w: 24.0, h: 24.0, fill: "#0A0716" });
typo_objects.push(#{ type: "rect", name: "QR Core TR", x: qrx + qrs - 28.0, y: qry + 16.0, w: 12.0, h: 12.0, fill: "#FFFFFF" });
typo_objects.push(#{ type: "rect", name: "QR Dot TR", x: qrx + qrs - 25.0, y: qry + 19.0, w: 6.0, h: 6.0, fill: "#7928CA" });

typo_objects.push(#{ type: "rect", name: "QR Corner BL", x: qrx + 10.0, y: qry + qrs - 34.0, w: 24.0, h: 24.0, fill: "#0A0716" });
typo_objects.push(#{ type: "rect", name: "QR Core BL", x: qrx + 16.0, y: qry + qrs - 28.0, w: 12.0, h: 12.0, fill: "#FFFFFF" });
typo_objects.push(#{ type: "rect", name: "QR Dot BL", x: qrx + 19.0, y: qry + qrs - 25.0, w: 6.0, h: 6.0, fill: "#7928CA" });

// Center data matrix clusters
typo_objects.push(#{ type: "rect", name: "QR Data 1", x: qrx + 42.0, y: qry + 42.0, w: 16.0, h: 16.0, fill: "#0A0716" });
typo_objects.push(#{ type: "rect", name: "QR Data 2", x: qrx + 62.0, y: qry + 48.0, w: 12.0, h: 24.0, fill: "#7928CA" });
typo_objects.push(#{ type: "rect", name: "QR Data 3", x: qrx + 42.0, y: qry + 66.0, w: 20.0, h: 10.0, fill: "#0A0716" });

// ───────────────────────────────────────────────────────────────────
// FINAL DOCUMENT ASSEMBLY
// ───────────────────────────────────────────────────────────────────
#{
    name: "HIRARI SOUND 2026 — Festival Poster",
    width: W,
    height: H,
    layers: [
        #{ name: "1. Atmosphere & Cosmic Space", opacity: 1.0, objects: bg_objects },
        #{ name: "2. Waveforms & Bézier Ribbons", opacity: 1.0, objects: waves_objects },
        #{ name: "3. Masked Sonic Core & Emblem", opacity: 1.0, objects: emblem_objects },
        #{ name: "4. Typography & Festival Info", opacity: 1.0, objects: typo_objects }
    ]
}
