use arbitrary_int::u5;
use bytemuck::{Pod, Zeroable};

use super::EntryType;

pub const UPCASE_TABLE: [u16; 2918] = [
    0x0, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x10, 0x11,
    0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21,
    0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31,
    0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f, 0x40, 0x41,
    0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51,
    0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f, 0x60, 0x41,
    0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51,
    0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x7b, 0x7c, 0x7d, 0x7e, 0x7f, 0x80, 0x81,
    0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x90, 0x91,
    0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f, 0xa0, 0xa1,
    0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xab, 0xac, 0xad, 0xae, 0xaf, 0xb0, 0xb1,
    0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xbb, 0xbc, 0xbd, 0xbe, 0xbf, 0xc0, 0xc1,
    0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd, 0xce, 0xcf, 0xd0, 0xd1,
    0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xdf, 0xc0, 0xc1,
    0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd, 0xce, 0xcf, 0xd0, 0xd1,
    0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xf7, 0xd8, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0x178, 0x100,
    0x100, 0x102, 0x102, 0x104, 0x104, 0x106, 0x106, 0x108, 0x108, 0x10a, 0x10a, 0x10c, 0x10c,
    0x10e, 0x10e, 0x110, 0x110, 0x112, 0x112, 0x114, 0x114, 0x116, 0x116, 0x118, 0x118, 0x11a,
    0x11a, 0x11c, 0x11c, 0x11e, 0x11e, 0x120, 0x120, 0x122, 0x122, 0x124, 0x124, 0x126, 0x126,
    0x128, 0x128, 0x12a, 0x12a, 0x12c, 0x12c, 0x12e, 0x12e, 0x130, 0x131, 0x132, 0x132, 0x134,
    0x134, 0x136, 0x136, 0x138, 0x139, 0x139, 0x13b, 0x13b, 0x13d, 0x13d, 0x13f, 0x13f, 0x141,
    0x141, 0x143, 0x143, 0x145, 0x145, 0x147, 0x147, 0x149, 0x14a, 0x14a, 0x14c, 0x14c, 0x14e,
    0x14e, 0x150, 0x150, 0x152, 0x152, 0x154, 0x154, 0x156, 0x156, 0x158, 0x158, 0x15a, 0x15a,
    0x15c, 0x15c, 0x15e, 0x15e, 0x160, 0x160, 0x162, 0x162, 0x164, 0x164, 0x166, 0x166, 0x168,
    0x168, 0x16a, 0x16a, 0x16c, 0x16c, 0x16e, 0x16e, 0x170, 0x170, 0x172, 0x172, 0x174, 0x174,
    0x176, 0x176, 0x178, 0x179, 0x179, 0x17b, 0x17b, 0x17d, 0x17d, 0x17f, 0x243, 0x181, 0x182,
    0x182, 0x184, 0x184, 0x186, 0x187, 0x187, 0x189, 0x18a, 0x18b, 0x18b, 0x18d, 0x18e, 0x18f,
    0x190, 0x191, 0x191, 0x193, 0x194, 0x1f6, 0x196, 0x197, 0x198, 0x198, 0x23d, 0x19b, 0x19c,
    0x19d, 0x220, 0x19f, 0x1a0, 0x1a0, 0x1a2, 0x1a2, 0x1a4, 0x1a4, 0x1a6, 0x1a7, 0x1a7, 0x1a9,
    0x1aa, 0x1ab, 0x1ac, 0x1ac, 0x1ae, 0x1af, 0x1af, 0x1b1, 0x1b2, 0x1b3, 0x1b3, 0x1b5, 0x1b5,
    0x1b7, 0x1b8, 0x1b8, 0x1ba, 0x1bb, 0x1bc, 0x1bc, 0x1be, 0x1f7, 0x1c0, 0x1c1, 0x1c2, 0x1c3,
    0x1c4, 0x1c5, 0x1c4, 0x1c7, 0x1c8, 0x1c7, 0x1ca, 0x1cb, 0x1ca, 0x1cd, 0x1cd, 0x1cf, 0x1cf,
    0x1d1, 0x1d1, 0x1d3, 0x1d3, 0x1d5, 0x1d5, 0x1d7, 0x1d7, 0x1d9, 0x1d9, 0x1db, 0x1db, 0x18e,
    0x1de, 0x1de, 0x1e0, 0x1e0, 0x1e2, 0x1e2, 0x1e4, 0x1e4, 0x1e6, 0x1e6, 0x1e8, 0x1e8, 0x1ea,
    0x1ea, 0x1ec, 0x1ec, 0x1ee, 0x1ee, 0x1f0, 0x1f1, 0x1f2, 0x1f1, 0x1f4, 0x1f4, 0x1f6, 0x1f7,
    0x1f8, 0x1f8, 0x1fa, 0x1fa, 0x1fc, 0x1fc, 0x1fe, 0x1fe, 0x200, 0x200, 0x202, 0x202, 0x204,
    0x204, 0x206, 0x206, 0x208, 0x208, 0x20a, 0x20a, 0x20c, 0x20c, 0x20e, 0x20e, 0x210, 0x210,
    0x212, 0x212, 0x214, 0x214, 0x216, 0x216, 0x218, 0x218, 0x21a, 0x21a, 0x21c, 0x21c, 0x21e,
    0x21e, 0x220, 0x221, 0x222, 0x222, 0x224, 0x224, 0x226, 0x226, 0x228, 0x228, 0x22a, 0x22a,
    0x22c, 0x22c, 0x22e, 0x22e, 0x230, 0x230, 0x232, 0x232, 0x234, 0x235, 0x236, 0x237, 0x238,
    0x239, 0x2c65, 0x23b, 0x23b, 0x23d, 0x2c66, 0x23f, 0x240, 0x241, 0x241, 0x243, 0x244, 0x245,
    0x246, 0x246, 0x248, 0x248, 0x24a, 0x24a, 0x24c, 0x24c, 0x24e, 0x24e, 0x250, 0x251, 0x252,
    0x181, 0x186, 0x255, 0x189, 0x18a, 0x258, 0x18f, 0x25a, 0x190, 0x25c, 0x25d, 0x25e, 0x25f,
    0x193, 0x261, 0x262, 0x194, 0x264, 0x265, 0x266, 0x267, 0x197, 0x196, 0x26a, 0x2c62, 0x26c,
    0x26d, 0x26e, 0x19c, 0x270, 0x271, 0x19d, 0x273, 0x274, 0x19f, 0x276, 0x277, 0x278, 0x279,
    0x27a, 0x27b, 0x27c, 0x2c64, 0x27e, 0x27f, 0x1a6, 0x281, 0x282, 0x1a9, 0x284, 0x285, 0x286,
    0x287, 0x1ae, 0x244, 0x1b1, 0x1b2, 0x245, 0x28d, 0x28e, 0x28f, 0x290, 0x291, 0x1b7, 0x293,
    0x294, 0x295, 0x296, 0x297, 0x298, 0x299, 0x29a, 0x29b, 0x29c, 0x29d, 0x29e, 0x29f, 0x2a0,
    0x2a1, 0x2a2, 0x2a3, 0x2a4, 0x2a5, 0x2a6, 0x2a7, 0x2a8, 0x2a9, 0x2aa, 0x2ab, 0x2ac, 0x2ad,
    0x2ae, 0x2af, 0x2b0, 0x2b1, 0x2b2, 0x2b3, 0x2b4, 0x2b5, 0x2b6, 0x2b7, 0x2b8, 0x2b9, 0x2ba,
    0x2bb, 0x2bc, 0x2bd, 0x2be, 0x2bf, 0x2c0, 0x2c1, 0x2c2, 0x2c3, 0x2c4, 0x2c5, 0x2c6, 0x2c7,
    0x2c8, 0x2c9, 0x2ca, 0x2cb, 0x2cc, 0x2cd, 0x2ce, 0x2cf, 0x2d0, 0x2d1, 0x2d2, 0x2d3, 0x2d4,
    0x2d5, 0x2d6, 0x2d7, 0x2d8, 0x2d9, 0x2da, 0x2db, 0x2dc, 0x2dd, 0x2de, 0x2df, 0x2e0, 0x2e1,
    0x2e2, 0x2e3, 0x2e4, 0x2e5, 0x2e6, 0x2e7, 0x2e8, 0x2e9, 0x2ea, 0x2eb, 0x2ec, 0x2ed, 0x2ee,
    0x2ef, 0x2f0, 0x2f1, 0x2f2, 0x2f3, 0x2f4, 0x2f5, 0x2f6, 0x2f7, 0x2f8, 0x2f9, 0x2fa, 0x2fb,
    0x2fc, 0x2fd, 0x2fe, 0x2ff, 0x300, 0x301, 0x302, 0x303, 0x304, 0x305, 0x306, 0x307, 0x308,
    0x309, 0x30a, 0x30b, 0x30c, 0x30d, 0x30e, 0x30f, 0x310, 0x311, 0x312, 0x313, 0x314, 0x315,
    0x316, 0x317, 0x318, 0x319, 0x31a, 0x31b, 0x31c, 0x31d, 0x31e, 0x31f, 0x320, 0x321, 0x322,
    0x323, 0x324, 0x325, 0x326, 0x327, 0x328, 0x329, 0x32a, 0x32b, 0x32c, 0x32d, 0x32e, 0x32f,
    0x330, 0x331, 0x332, 0x333, 0x334, 0x335, 0x336, 0x337, 0x338, 0x339, 0x33a, 0x33b, 0x33c,
    0x33d, 0x33e, 0x33f, 0x340, 0x341, 0x342, 0x343, 0x344, 0x345, 0x346, 0x347, 0x348, 0x349,
    0x34a, 0x34b, 0x34c, 0x34d, 0x34e, 0x34f, 0x350, 0x351, 0x352, 0x353, 0x354, 0x355, 0x356,
    0x357, 0x358, 0x359, 0x35a, 0x35b, 0x35c, 0x35d, 0x35e, 0x35f, 0x360, 0x361, 0x362, 0x363,
    0x364, 0x365, 0x366, 0x367, 0x368, 0x369, 0x36a, 0x36b, 0x36c, 0x36d, 0x36e, 0x36f, 0x370,
    0x371, 0x372, 0x373, 0x374, 0x375, 0x376, 0x377, 0x378, 0x379, 0x37a, 0x3fd, 0x3fe, 0x3ff,
    0x37e, 0x37f, 0x380, 0x381, 0x382, 0x383, 0x384, 0x385, 0x386, 0x387, 0x388, 0x389, 0x38a,
    0x38b, 0x38c, 0x38d, 0x38e, 0x38f, 0x390, 0x391, 0x392, 0x393, 0x394, 0x395, 0x396, 0x397,
    0x398, 0x399, 0x39a, 0x39b, 0x39c, 0x39d, 0x39e, 0x39f, 0x3a0, 0x3a1, 0x3a2, 0x3a3, 0x3a4,
    0x3a5, 0x3a6, 0x3a7, 0x3a8, 0x3a9, 0x3aa, 0x3ab, 0x386, 0x388, 0x389, 0x38a, 0x3b0, 0x391,
    0x392, 0x393, 0x394, 0x395, 0x396, 0x397, 0x398, 0x399, 0x39a, 0x39b, 0x39c, 0x39d, 0x39e,
    0x39f, 0x3a0, 0x3a1, 0x3a3, 0x3a3, 0x3a4, 0x3a5, 0x3a6, 0x3a7, 0x3a8, 0x3a9, 0x3aa, 0x3ab,
    0x38c, 0x38e, 0x38f, 0x3cf, 0x3d0, 0x3d1, 0x3d2, 0x3d3, 0x3d4, 0x3d5, 0x3d6, 0x3d7, 0x3d8,
    0x3d8, 0x3da, 0x3da, 0x3dc, 0x3dc, 0x3de, 0x3de, 0x3e0, 0x3e0, 0x3e2, 0x3e2, 0x3e4, 0x3e4,
    0x3e6, 0x3e6, 0x3e8, 0x3e8, 0x3ea, 0x3ea, 0x3ec, 0x3ec, 0x3ee, 0x3ee, 0x3f0, 0x3f1, 0x3f9,
    0x3f3, 0x3f4, 0x3f5, 0x3f6, 0x3f7, 0x3f7, 0x3f9, 0x3fa, 0x3fa, 0x3fc, 0x3fd, 0x3fe, 0x3ff,
    0x400, 0x401, 0x402, 0x403, 0x404, 0x405, 0x406, 0x407, 0x408, 0x409, 0x40a, 0x40b, 0x40c,
    0x40d, 0x40e, 0x40f, 0x410, 0x411, 0x412, 0x413, 0x414, 0x415, 0x416, 0x417, 0x418, 0x419,
    0x41a, 0x41b, 0x41c, 0x41d, 0x41e, 0x41f, 0x420, 0x421, 0x422, 0x423, 0x424, 0x425, 0x426,
    0x427, 0x428, 0x429, 0x42a, 0x42b, 0x42c, 0x42d, 0x42e, 0x42f, 0x410, 0x411, 0x412, 0x413,
    0x414, 0x415, 0x416, 0x417, 0x418, 0x419, 0x41a, 0x41b, 0x41c, 0x41d, 0x41e, 0x41f, 0x420,
    0x421, 0x422, 0x423, 0x424, 0x425, 0x426, 0x427, 0x428, 0x429, 0x42a, 0x42b, 0x42c, 0x42d,
    0x42e, 0x42f, 0x400, 0x401, 0x402, 0x403, 0x404, 0x405, 0x406, 0x407, 0x408, 0x409, 0x40a,
    0x40b, 0x40c, 0x40d, 0x40e, 0x40f, 0x460, 0x460, 0x462, 0x462, 0x464, 0x464, 0x466, 0x466,
    0x468, 0x468, 0x46a, 0x46a, 0x46c, 0x46c, 0x46e, 0x46e, 0x470, 0x470, 0x472, 0x472, 0x474,
    0x474, 0x476, 0x476, 0x478, 0x478, 0x47a, 0x47a, 0x47c, 0x47c, 0x47e, 0x47e, 0x480, 0x480,
    0x482, 0x483, 0x484, 0x485, 0x486, 0x487, 0x488, 0x489, 0x48a, 0x48a, 0x48c, 0x48c, 0x48e,
    0x48e, 0x490, 0x490, 0x492, 0x492, 0x494, 0x494, 0x496, 0x496, 0x498, 0x498, 0x49a, 0x49a,
    0x49c, 0x49c, 0x49e, 0x49e, 0x4a0, 0x4a0, 0x4a2, 0x4a2, 0x4a4, 0x4a4, 0x4a6, 0x4a6, 0x4a8,
    0x4a8, 0x4aa, 0x4aa, 0x4ac, 0x4ac, 0x4ae, 0x4ae, 0x4b0, 0x4b0, 0x4b2, 0x4b2, 0x4b4, 0x4b4,
    0x4b6, 0x4b6, 0x4b8, 0x4b8, 0x4ba, 0x4ba, 0x4bc, 0x4bc, 0x4be, 0x4be, 0x4c0, 0x4c1, 0x4c1,
    0x4c3, 0x4c3, 0x4c5, 0x4c5, 0x4c7, 0x4c7, 0x4c9, 0x4c9, 0x4cb, 0x4cb, 0x4cd, 0x4cd, 0x4c0,
    0x4d0, 0x4d0, 0x4d2, 0x4d2, 0x4d4, 0x4d4, 0x4d6, 0x4d6, 0x4d8, 0x4d8, 0x4da, 0x4da, 0x4dc,
    0x4dc, 0x4de, 0x4de, 0x4e0, 0x4e0, 0x4e2, 0x4e2, 0x4e4, 0x4e4, 0x4e6, 0x4e6, 0x4e8, 0x4e8,
    0x4ea, 0x4ea, 0x4ec, 0x4ec, 0x4ee, 0x4ee, 0x4f0, 0x4f0, 0x4f2, 0x4f2, 0x4f4, 0x4f4, 0x4f6,
    0x4f6, 0x4f8, 0x4f8, 0x4fa, 0x4fa, 0x4fc, 0x4fc, 0x4fe, 0x4fe, 0x500, 0x500, 0x502, 0x502,
    0x504, 0x504, 0x506, 0x506, 0x508, 0x508, 0x50a, 0x50a, 0x50c, 0x50c, 0x50e, 0x50e, 0x510,
    0x510, 0x512, 0x512, 0x514, 0x515, 0x516, 0x517, 0x518, 0x519, 0x51a, 0x51b, 0x51c, 0x51d,
    0x51e, 0x51f, 0x520, 0x521, 0x522, 0x523, 0x524, 0x525, 0x526, 0x527, 0x528, 0x529, 0x52a,
    0x52b, 0x52c, 0x52d, 0x52e, 0x52f, 0x530, 0x531, 0x532, 0x533, 0x534, 0x535, 0x536, 0x537,
    0x538, 0x539, 0x53a, 0x53b, 0x53c, 0x53d, 0x53e, 0x53f, 0x540, 0x541, 0x542, 0x543, 0x544,
    0x545, 0x546, 0x547, 0x548, 0x549, 0x54a, 0x54b, 0x54c, 0x54d, 0x54e, 0x54f, 0x550, 0x551,
    0x552, 0x553, 0x554, 0x555, 0x556, 0x557, 0x558, 0x559, 0x55a, 0x55b, 0x55c, 0x55d, 0x55e,
    0x55f, 0x560, 0x531, 0x532, 0x533, 0x534, 0x535, 0x536, 0x537, 0x538, 0x539, 0x53a, 0x53b,
    0x53c, 0x53d, 0x53e, 0x53f, 0x540, 0x541, 0x542, 0x543, 0x544, 0x545, 0x546, 0x547, 0x548,
    0x549, 0x54a, 0x54b, 0x54c, 0x54d, 0x54e, 0x54f, 0x550, 0x551, 0x552, 0x553, 0x554, 0x555,
    0x556, 0xffff, 0x17f6, 0x2c63, 0x1d7e, 0x1d7f, 0x1d80, 0x1d81, 0x1d82, 0x1d83, 0x1d84, 0x1d85,
    0x1d86, 0x1d87, 0x1d88, 0x1d89, 0x1d8a, 0x1d8b, 0x1d8c, 0x1d8d, 0x1d8e, 0x1d8f, 0x1d90, 0x1d91,
    0x1d92, 0x1d93, 0x1d94, 0x1d95, 0x1d96, 0x1d97, 0x1d98, 0x1d99, 0x1d9a, 0x1d9b, 0x1d9c, 0x1d9d,
    0x1d9e, 0x1d9f, 0x1da0, 0x1da1, 0x1da2, 0x1da3, 0x1da4, 0x1da5, 0x1da6, 0x1da7, 0x1da8, 0x1da9,
    0x1daa, 0x1dab, 0x1dac, 0x1dad, 0x1dae, 0x1daf, 0x1db0, 0x1db1, 0x1db2, 0x1db3, 0x1db4, 0x1db5,
    0x1db6, 0x1db7, 0x1db8, 0x1db9, 0x1dba, 0x1dbb, 0x1dbc, 0x1dbd, 0x1dbe, 0x1dbf, 0x1dc0, 0x1dc1,
    0x1dc2, 0x1dc3, 0x1dc4, 0x1dc5, 0x1dc6, 0x1dc7, 0x1dc8, 0x1dc9, 0x1dca, 0x1dcb, 0x1dcc, 0x1dcd,
    0x1dce, 0x1dcf, 0x1dd0, 0x1dd1, 0x1dd2, 0x1dd3, 0x1dd4, 0x1dd5, 0x1dd6, 0x1dd7, 0x1dd8, 0x1dd9,
    0x1dda, 0x1ddb, 0x1ddc, 0x1ddd, 0x1dde, 0x1ddf, 0x1de0, 0x1de1, 0x1de2, 0x1de3, 0x1de4, 0x1de5,
    0x1de6, 0x1de7, 0x1de8, 0x1de9, 0x1dea, 0x1deb, 0x1dec, 0x1ded, 0x1dee, 0x1def, 0x1df0, 0x1df1,
    0x1df2, 0x1df3, 0x1df4, 0x1df5, 0x1df6, 0x1df7, 0x1df8, 0x1df9, 0x1dfa, 0x1dfb, 0x1dfc, 0x1dfd,
    0x1dfe, 0x1dff, 0x1e00, 0x1e00, 0x1e02, 0x1e02, 0x1e04, 0x1e04, 0x1e06, 0x1e06, 0x1e08, 0x1e08,
    0x1e0a, 0x1e0a, 0x1e0c, 0x1e0c, 0x1e0e, 0x1e0e, 0x1e10, 0x1e10, 0x1e12, 0x1e12, 0x1e14, 0x1e14,
    0x1e16, 0x1e16, 0x1e18, 0x1e18, 0x1e1a, 0x1e1a, 0x1e1c, 0x1e1c, 0x1e1e, 0x1e1e, 0x1e20, 0x1e20,
    0x1e22, 0x1e22, 0x1e24, 0x1e24, 0x1e26, 0x1e26, 0x1e28, 0x1e28, 0x1e2a, 0x1e2a, 0x1e2c, 0x1e2c,
    0x1e2e, 0x1e2e, 0x1e30, 0x1e30, 0x1e32, 0x1e32, 0x1e34, 0x1e34, 0x1e36, 0x1e36, 0x1e38, 0x1e38,
    0x1e3a, 0x1e3a, 0x1e3c, 0x1e3c, 0x1e3e, 0x1e3e, 0x1e40, 0x1e40, 0x1e42, 0x1e42, 0x1e44, 0x1e44,
    0x1e46, 0x1e46, 0x1e48, 0x1e48, 0x1e4a, 0x1e4a, 0x1e4c, 0x1e4c, 0x1e4e, 0x1e4e, 0x1e50, 0x1e50,
    0x1e52, 0x1e52, 0x1e54, 0x1e54, 0x1e56, 0x1e56, 0x1e58, 0x1e58, 0x1e5a, 0x1e5a, 0x1e5c, 0x1e5c,
    0x1e5e, 0x1e5e, 0x1e60, 0x1e60, 0x1e62, 0x1e62, 0x1e64, 0x1e64, 0x1e66, 0x1e66, 0x1e68, 0x1e68,
    0x1e6a, 0x1e6a, 0x1e6c, 0x1e6c, 0x1e6e, 0x1e6e, 0x1e70, 0x1e70, 0x1e72, 0x1e72, 0x1e74, 0x1e74,
    0x1e76, 0x1e76, 0x1e78, 0x1e78, 0x1e7a, 0x1e7a, 0x1e7c, 0x1e7c, 0x1e7e, 0x1e7e, 0x1e80, 0x1e80,
    0x1e82, 0x1e82, 0x1e84, 0x1e84, 0x1e86, 0x1e86, 0x1e88, 0x1e88, 0x1e8a, 0x1e8a, 0x1e8c, 0x1e8c,
    0x1e8e, 0x1e8e, 0x1e90, 0x1e90, 0x1e92, 0x1e92, 0x1e94, 0x1e94, 0x1e96, 0x1e97, 0x1e98, 0x1e99,
    0x1e9a, 0x1e9b, 0x1e9c, 0x1e9d, 0x1e9e, 0x1e9f, 0x1ea0, 0x1ea0, 0x1ea2, 0x1ea2, 0x1ea4, 0x1ea4,
    0x1ea6, 0x1ea6, 0x1ea8, 0x1ea8, 0x1eaa, 0x1eaa, 0x1eac, 0x1eac, 0x1eae, 0x1eae, 0x1eb0, 0x1eb0,
    0x1eb2, 0x1eb2, 0x1eb4, 0x1eb4, 0x1eb6, 0x1eb6, 0x1eb8, 0x1eb8, 0x1eba, 0x1eba, 0x1ebc, 0x1ebc,
    0x1ebe, 0x1ebe, 0x1ec0, 0x1ec0, 0x1ec2, 0x1ec2, 0x1ec4, 0x1ec4, 0x1ec6, 0x1ec6, 0x1ec8, 0x1ec8,
    0x1eca, 0x1eca, 0x1ecc, 0x1ecc, 0x1ece, 0x1ece, 0x1ed0, 0x1ed0, 0x1ed2, 0x1ed2, 0x1ed4, 0x1ed4,
    0x1ed6, 0x1ed6, 0x1ed8, 0x1ed8, 0x1eda, 0x1eda, 0x1edc, 0x1edc, 0x1ede, 0x1ede, 0x1ee0, 0x1ee0,
    0x1ee2, 0x1ee2, 0x1ee4, 0x1ee4, 0x1ee6, 0x1ee6, 0x1ee8, 0x1ee8, 0x1eea, 0x1eea, 0x1eec, 0x1eec,
    0x1eee, 0x1eee, 0x1ef0, 0x1ef0, 0x1ef2, 0x1ef2, 0x1ef4, 0x1ef4, 0x1ef6, 0x1ef6, 0x1ef8, 0x1ef8,
    0x1efa, 0x1efb, 0x1efc, 0x1efd, 0x1efe, 0x1eff, 0x1f08, 0x1f09, 0x1f0a, 0x1f0b, 0x1f0c, 0x1f0d,
    0x1f0e, 0x1f0f, 0x1f08, 0x1f09, 0x1f0a, 0x1f0b, 0x1f0c, 0x1f0d, 0x1f0e, 0x1f0f, 0x1f18, 0x1f19,
    0x1f1a, 0x1f1b, 0x1f1c, 0x1f1d, 0x1f16, 0x1f17, 0x1f18, 0x1f19, 0x1f1a, 0x1f1b, 0x1f1c, 0x1f1d,
    0x1f1e, 0x1f1f, 0x1f28, 0x1f29, 0x1f2a, 0x1f2b, 0x1f2c, 0x1f2d, 0x1f2e, 0x1f2f, 0x1f28, 0x1f29,
    0x1f2a, 0x1f2b, 0x1f2c, 0x1f2d, 0x1f2e, 0x1f2f, 0x1f38, 0x1f39, 0x1f3a, 0x1f3b, 0x1f3c, 0x1f3d,
    0x1f3e, 0x1f3f, 0x1f38, 0x1f39, 0x1f3a, 0x1f3b, 0x1f3c, 0x1f3d, 0x1f3e, 0x1f3f, 0x1f48, 0x1f49,
    0x1f4a, 0x1f4b, 0x1f4c, 0x1f4d, 0x1f46, 0x1f47, 0x1f48, 0x1f49, 0x1f4a, 0x1f4b, 0x1f4c, 0x1f4d,
    0x1f4e, 0x1f4f, 0x1f50, 0x1f59, 0x1f52, 0x1f5b, 0x1f54, 0x1f5d, 0x1f56, 0x1f5f, 0x1f58, 0x1f59,
    0x1f5a, 0x1f5b, 0x1f5c, 0x1f5d, 0x1f5e, 0x1f5f, 0x1f68, 0x1f69, 0x1f6a, 0x1f6b, 0x1f6c, 0x1f6d,
    0x1f6e, 0x1f6f, 0x1f68, 0x1f69, 0x1f6a, 0x1f6b, 0x1f6c, 0x1f6d, 0x1f6e, 0x1f6f, 0x1fba, 0x1fbb,
    0x1fc8, 0x1fc9, 0x1fca, 0x1fcb, 0x1fda, 0x1fdb, 0x1ff8, 0x1ff9, 0x1fea, 0x1feb, 0x1ffa, 0x1ffb,
    0x1f7e, 0x1f7f, 0x1f88, 0x1f89, 0x1f8a, 0x1f8b, 0x1f8c, 0x1f8d, 0x1f8e, 0x1f8f, 0x1f88, 0x1f89,
    0x1f8a, 0x1f8b, 0x1f8c, 0x1f8d, 0x1f8e, 0x1f8f, 0x1f98, 0x1f99, 0x1f9a, 0x1f9b, 0x1f9c, 0x1f9d,
    0x1f9e, 0x1f9f, 0x1f98, 0x1f99, 0x1f9a, 0x1f9b, 0x1f9c, 0x1f9d, 0x1f9e, 0x1f9f, 0x1fa8, 0x1fa9,
    0x1faa, 0x1fab, 0x1fac, 0x1fad, 0x1fae, 0x1faf, 0x1fa8, 0x1fa9, 0x1faa, 0x1fab, 0x1fac, 0x1fad,
    0x1fae, 0x1faf, 0x1fb8, 0x1fb9, 0x1fb2, 0x1fbc, 0x1fb4, 0x1fb5, 0x1fb6, 0x1fb7, 0x1fb8, 0x1fb9,
    0x1fba, 0x1fbb, 0x1fbc, 0x1fbd, 0x1fbe, 0x1fbf, 0x1fc0, 0x1fc1, 0x1fc2, 0x1fc3, 0x1fc4, 0x1fc5,
    0x1fc6, 0x1fc7, 0x1fc8, 0x1fc9, 0x1fca, 0x1fcb, 0x1fc3, 0x1fcd, 0x1fce, 0x1fcf, 0x1fd8, 0x1fd9,
    0x1fd2, 0x1fd3, 0x1fd4, 0x1fd5, 0x1fd6, 0x1fd7, 0x1fd8, 0x1fd9, 0x1fda, 0x1fdb, 0x1fdc, 0x1fdd,
    0x1fde, 0x1fdf, 0x1fe8, 0x1fe9, 0x1fe2, 0x1fe3, 0x1fe4, 0x1fec, 0x1fe6, 0x1fe7, 0x1fe8, 0x1fe9,
    0x1fea, 0x1feb, 0x1fec, 0x1fed, 0x1fee, 0x1fef, 0x1ff0, 0x1ff1, 0x1ff2, 0x1ff3, 0x1ff4, 0x1ff5,
    0x1ff6, 0x1ff7, 0x1ff8, 0x1ff9, 0x1ffa, 0x1ffb, 0x1ff3, 0x1ffd, 0x1ffe, 0x1fff, 0x2000, 0x2001,
    0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x200b, 0x200c, 0x200d,
    0x200e, 0x200f, 0x2010, 0x2011, 0x2012, 0x2013, 0x2014, 0x2015, 0x2016, 0x2017, 0x2018, 0x2019,
    0x201a, 0x201b, 0x201c, 0x201d, 0x201e, 0x201f, 0x2020, 0x2021, 0x2022, 0x2023, 0x2024, 0x2025,
    0x2026, 0x2027, 0x2028, 0x2029, 0x202a, 0x202b, 0x202c, 0x202d, 0x202e, 0x202f, 0x2030, 0x2031,
    0x2032, 0x2033, 0x2034, 0x2035, 0x2036, 0x2037, 0x2038, 0x2039, 0x203a, 0x203b, 0x203c, 0x203d,
    0x203e, 0x203f, 0x2040, 0x2041, 0x2042, 0x2043, 0x2044, 0x2045, 0x2046, 0x2047, 0x2048, 0x2049,
    0x204a, 0x204b, 0x204c, 0x204d, 0x204e, 0x204f, 0x2050, 0x2051, 0x2052, 0x2053, 0x2054, 0x2055,
    0x2056, 0x2057, 0x2058, 0x2059, 0x205a, 0x205b, 0x205c, 0x205d, 0x205e, 0x205f, 0x2060, 0x2061,
    0x2062, 0x2063, 0x2064, 0x2065, 0x2066, 0x2067, 0x2068, 0x2069, 0x206a, 0x206b, 0x206c, 0x206d,
    0x206e, 0x206f, 0x2070, 0x2071, 0x2072, 0x2073, 0x2074, 0x2075, 0x2076, 0x2077, 0x2078, 0x2079,
    0x207a, 0x207b, 0x207c, 0x207d, 0x207e, 0x207f, 0x2080, 0x2081, 0x2082, 0x2083, 0x2084, 0x2085,
    0x2086, 0x2087, 0x2088, 0x2089, 0x208a, 0x208b, 0x208c, 0x208d, 0x208e, 0x208f, 0x2090, 0x2091,
    0x2092, 0x2093, 0x2094, 0x2095, 0x2096, 0x2097, 0x2098, 0x2099, 0x209a, 0x209b, 0x209c, 0x209d,
    0x209e, 0x209f, 0x20a0, 0x20a1, 0x20a2, 0x20a3, 0x20a4, 0x20a5, 0x20a6, 0x20a7, 0x20a8, 0x20a9,
    0x20aa, 0x20ab, 0x20ac, 0x20ad, 0x20ae, 0x20af, 0x20b0, 0x20b1, 0x20b2, 0x20b3, 0x20b4, 0x20b5,
    0x20b6, 0x20b7, 0x20b8, 0x20b9, 0x20ba, 0x20bb, 0x20bc, 0x20bd, 0x20be, 0x20bf, 0x20c0, 0x20c1,
    0x20c2, 0x20c3, 0x20c4, 0x20c5, 0x20c6, 0x20c7, 0x20c8, 0x20c9, 0x20ca, 0x20cb, 0x20cc, 0x20cd,
    0x20ce, 0x20cf, 0x20d0, 0x20d1, 0x20d2, 0x20d3, 0x20d4, 0x20d5, 0x20d6, 0x20d7, 0x20d8, 0x20d9,
    0x20da, 0x20db, 0x20dc, 0x20dd, 0x20de, 0x20df, 0x20e0, 0x20e1, 0x20e2, 0x20e3, 0x20e4, 0x20e5,
    0x20e6, 0x20e7, 0x20e8, 0x20e9, 0x20ea, 0x20eb, 0x20ec, 0x20ed, 0x20ee, 0x20ef, 0x20f0, 0x20f1,
    0x20f2, 0x20f3, 0x20f4, 0x20f5, 0x20f6, 0x20f7, 0x20f8, 0x20f9, 0x20fa, 0x20fb, 0x20fc, 0x20fd,
    0x20fe, 0x20ff, 0x2100, 0x2101, 0x2102, 0x2103, 0x2104, 0x2105, 0x2106, 0x2107, 0x2108, 0x2109,
    0x210a, 0x210b, 0x210c, 0x210d, 0x210e, 0x210f, 0x2110, 0x2111, 0x2112, 0x2113, 0x2114, 0x2115,
    0x2116, 0x2117, 0x2118, 0x2119, 0x211a, 0x211b, 0x211c, 0x211d, 0x211e, 0x211f, 0x2120, 0x2121,
    0x2122, 0x2123, 0x2124, 0x2125, 0x2126, 0x2127, 0x2128, 0x2129, 0x212a, 0x212b, 0x212c, 0x212d,
    0x212e, 0x212f, 0x2130, 0x2131, 0x2132, 0x2133, 0x2134, 0x2135, 0x2136, 0x2137, 0x2138, 0x2139,
    0x213a, 0x213b, 0x213c, 0x213d, 0x213e, 0x213f, 0x2140, 0x2141, 0x2142, 0x2143, 0x2144, 0x2145,
    0x2146, 0x2147, 0x2148, 0x2149, 0x214a, 0x214b, 0x214c, 0x214d, 0x2132, 0x214f, 0x2150, 0x2151,
    0x2152, 0x2153, 0x2154, 0x2155, 0x2156, 0x2157, 0x2158, 0x2159, 0x215a, 0x215b, 0x215c, 0x215d,
    0x215e, 0x215f, 0x2160, 0x2161, 0x2162, 0x2163, 0x2164, 0x2165, 0x2166, 0x2167, 0x2168, 0x2169,
    0x216a, 0x216b, 0x216c, 0x216d, 0x216e, 0x216f, 0x2160, 0x2161, 0x2162, 0x2163, 0x2164, 0x2165,
    0x2166, 0x2167, 0x2168, 0x2169, 0x216a, 0x216b, 0x216c, 0x216d, 0x216e, 0x216f, 0x2180, 0x2181,
    0x2182, 0x2183, 0x2183, 0xffff, 0x34b, 0x24b6, 0x24b7, 0x24b8, 0x24b9, 0x24ba, 0x24bb, 0x24bc,
    0x24bd, 0x24be, 0x24bf, 0x24c0, 0x24c1, 0x24c2, 0x24c3, 0x24c4, 0x24c5, 0x24c6, 0x24c7, 0x24c8,
    0x24c9, 0x24ca, 0x24cb, 0x24cc, 0x24cd, 0x24ce, 0x24cf, 0xffff, 0x746, 0x2c00, 0x2c01, 0x2c02,
    0x2c03, 0x2c04, 0x2c05, 0x2c06, 0x2c07, 0x2c08, 0x2c09, 0x2c0a, 0x2c0b, 0x2c0c, 0x2c0d, 0x2c0e,
    0x2c0f, 0x2c10, 0x2c11, 0x2c12, 0x2c13, 0x2c14, 0x2c15, 0x2c16, 0x2c17, 0x2c18, 0x2c19, 0x2c1a,
    0x2c1b, 0x2c1c, 0x2c1d, 0x2c1e, 0x2c1f, 0x2c20, 0x2c21, 0x2c22, 0x2c23, 0x2c24, 0x2c25, 0x2c26,
    0x2c27, 0x2c28, 0x2c29, 0x2c2a, 0x2c2b, 0x2c2c, 0x2c2d, 0x2c2e, 0x2c5f, 0x2c60, 0x2c60, 0x2c62,
    0x2c63, 0x2c64, 0x2c65, 0x2c66, 0x2c67, 0x2c67, 0x2c69, 0x2c69, 0x2c6b, 0x2c6b, 0x2c6d, 0x2c6e,
    0x2c6f, 0x2c70, 0x2c71, 0x2c72, 0x2c73, 0x2c74, 0x2c75, 0x2c75, 0x2c77, 0x2c78, 0x2c79, 0x2c7a,
    0x2c7b, 0x2c7c, 0x2c7d, 0x2c7e, 0x2c7f, 0x2c80, 0x2c80, 0x2c82, 0x2c82, 0x2c84, 0x2c84, 0x2c86,
    0x2c86, 0x2c88, 0x2c88, 0x2c8a, 0x2c8a, 0x2c8c, 0x2c8c, 0x2c8e, 0x2c8e, 0x2c90, 0x2c90, 0x2c92,
    0x2c92, 0x2c94, 0x2c94, 0x2c96, 0x2c96, 0x2c98, 0x2c98, 0x2c9a, 0x2c9a, 0x2c9c, 0x2c9c, 0x2c9e,
    0x2c9e, 0x2ca0, 0x2ca0, 0x2ca2, 0x2ca2, 0x2ca4, 0x2ca4, 0x2ca6, 0x2ca6, 0x2ca8, 0x2ca8, 0x2caa,
    0x2caa, 0x2cac, 0x2cac, 0x2cae, 0x2cae, 0x2cb0, 0x2cb0, 0x2cb2, 0x2cb2, 0x2cb4, 0x2cb4, 0x2cb6,
    0x2cb6, 0x2cb8, 0x2cb8, 0x2cba, 0x2cba, 0x2cbc, 0x2cbc, 0x2cbe, 0x2cbe, 0x2cc0, 0x2cc0, 0x2cc2,
    0x2cc2, 0x2cc4, 0x2cc4, 0x2cc6, 0x2cc6, 0x2cc8, 0x2cc8, 0x2cca, 0x2cca, 0x2ccc, 0x2ccc, 0x2cce,
    0x2cce, 0x2cd0, 0x2cd0, 0x2cd2, 0x2cd2, 0x2cd4, 0x2cd4, 0x2cd6, 0x2cd6, 0x2cd8, 0x2cd8, 0x2cda,
    0x2cda, 0x2cdc, 0x2cdc, 0x2cde, 0x2cde, 0x2ce0, 0x2ce0, 0x2ce2, 0x2ce2, 0x2ce4, 0x2ce5, 0x2ce6,
    0x2ce7, 0x2ce8, 0x2ce9, 0x2cea, 0x2ceb, 0x2cec, 0x2ced, 0x2cee, 0x2cef, 0x2cf0, 0x2cf1, 0x2cf2,
    0x2cf3, 0x2cf4, 0x2cf5, 0x2cf6, 0x2cf7, 0x2cf8, 0x2cf9, 0x2cfa, 0x2cfb, 0x2cfc, 0x2cfd, 0x2cfe,
    0x2cff, 0x10a0, 0x10a1, 0x10a2, 0x10a3, 0x10a4, 0x10a5, 0x10a6, 0x10a7, 0x10a8, 0x10a9, 0x10aa,
    0x10ab, 0x10ac, 0x10ad, 0x10ae, 0x10af, 0x10b0, 0x10b1, 0x10b2, 0x10b3, 0x10b4, 0x10b5, 0x10b6,
    0x10b7, 0x10b8, 0x10b9, 0x10ba, 0x10bb, 0x10bc, 0x10bd, 0x10be, 0x10bf, 0x10c0, 0x10c1, 0x10c2,
    0x10c3, 0x10c4, 0x10c5, 0xffff, 0xd21b, 0xff21, 0xff22, 0xff23, 0xff24, 0xff25, 0xff26, 0xff27,
    0xff28, 0xff29, 0xff2a, 0xff2b, 0xff2c, 0xff2d, 0xff2e, 0xff2f, 0xff30, 0xff31, 0xff32, 0xff33,
    0xff34, 0xff35, 0xff36, 0xff37, 0xff38, 0xff39, 0xff3a, 0xff5b, 0xff5c, 0xff5d, 0xff5e, 0xff5f,
    0xff60, 0xff61, 0xff62, 0xff63, 0xff64, 0xff65, 0xff66, 0xff67, 0xff68, 0xff69, 0xff6a, 0xff6b,
    0xff6c, 0xff6d, 0xff6e, 0xff6f, 0xff70, 0xff71, 0xff72, 0xff73, 0xff74, 0xff75, 0xff76, 0xff77,
    0xff78, 0xff79, 0xff7a, 0xff7b, 0xff7c, 0xff7d, 0xff7e, 0xff7f, 0xff80, 0xff81, 0xff82, 0xff83,
    0xff84, 0xff85, 0xff86, 0xff87, 0xff88, 0xff89, 0xff8a, 0xff8b, 0xff8c, 0xff8d, 0xff8e, 0xff8f,
    0xff90, 0xff91, 0xff92, 0xff93, 0xff94, 0xff95, 0xff96, 0xff97, 0xff98, 0xff99, 0xff9a, 0xff9b,
    0xff9c, 0xff9d, 0xff9e, 0xff9f, 0xffa0, 0xffa1, 0xffa2, 0xffa3, 0xffa4, 0xffa5, 0xffa6, 0xffa7,
    0xffa8, 0xffa9, 0xffaa, 0xffab, 0xffac, 0xffad, 0xffae, 0xffaf, 0xffb0, 0xffb1, 0xffb2, 0xffb3,
    0xffb4, 0xffb5, 0xffb6, 0xffb7, 0xffb8, 0xffb9, 0xffba, 0xffbb, 0xffbc, 0xffbd, 0xffbe, 0xffbf,
    0xffc0, 0xffc1, 0xffc2, 0xffc3, 0xffc4, 0xffc5, 0xffc6, 0xffc7, 0xffc8, 0xffc9, 0xffca, 0xffcb,
    0xffcc, 0xffcd, 0xffce, 0xffcf, 0xffd0, 0xffd1, 0xffd2, 0xffd3, 0xffd4, 0xffd5, 0xffd6, 0xffd7,
    0xffd8, 0xffd9, 0xffda, 0xffdb, 0xffdc, 0xffdd, 0xffde, 0xffdf, 0xffe0, 0xffe1, 0xffe2, 0xffe3,
    0xffe4, 0xffe5, 0xffe6, 0xffe7, 0xffe8, 0xffe9, 0xffea, 0xffeb, 0xffec, 0xffed, 0xffee, 0xffef,
    0xfff0, 0xfff1, 0xfff2, 0xfff3, 0xfff4, 0xfff5, 0xfff6, 0xfff7, 0xfff8, 0xfff9, 0xfffa, 0xfffb,
    0xfffc, 0xfffd, 0xfffe, 0xffff,
];

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct UpcaseTableDirectoryEntry {
    entry_type: EntryType,
    reserved_1: [u8; 3],
    table_checksum: u32,
    reserved_2: [u8; 12],
    first_cluster: u32,
    data_length: u64,
}

impl UpcaseTableDirectoryEntry {
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

impl Default for UpcaseTableDirectoryEntry {
    fn default() -> Self {
        Self {
            entry_type: EntryType::new_with_raw_value(0)
                .with_type_code(u5::new(1))
                .with_in_use(true), // 0x82
            reserved_1: [0; 3],
            table_checksum: 0xE619D30D,
            reserved_2: [0; 12],
            first_cluster: 3,
            data_length: 0x16CC,
        }
    }
}

pub fn upcased_file_name(file_name: &str) -> Vec<u16> {
    let mut upcased = Vec::new();

    for ch in file_name.encode_utf16() {
        upcased.push(match UPCASE_TABLE.get(usize::from(ch)).cloned() {
            Some(upper) => upper,
            None => ch,
        })
    }

    upcased
}

#[test]
fn upcasing() {
    let upcased_utf16 = upcased_file_name("Hello World");
    let upcased_utf8 = String::from_utf16(&upcased_utf16).unwrap();
    assert_eq!(upcased_utf8, "HELLO WORLD");
}
