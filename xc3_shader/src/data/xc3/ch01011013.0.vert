const int undef = 0;
layout(binding = 0, std140) uniform _support_buffer {
    uint alpha_test;
    uint is_bgra[8];
    precise vec4 viewport_inverse;
    precise vec4 viewport_size;
    int frag_scale_count;
    precise float render_scale[73];
    ivec4 tfe_offset;
    int tfe_vertex_count;
}support_buffer;
layout(binding = 5, std140) uniform _U_Mate {
    vec4 gWrkFl4[2];
    vec4 gWrkCol;
    vec4 gMatCol;
}U_Mate;
layout(binding = 6, std140) uniform _U_Toon2 {
    vec4 gToonParam[4];
    vec4 gHatchingParam[3];
}U_Toon2;
layout(binding = 4, std140) uniform _U_Static {
    vec4 gmView[3];
    vec4 gmProj[4];
    vec4 gmViewProj[4];
    vec4 gmInvView[3];
    vec4 gBilMat[3];
    vec4 gBilYJiku[3];
    vec4 gEtcParm;
    vec4 gViewYVec;
    vec4 gCDep;
    vec4 gDitVal;
    vec4 gPreMat[4];
    vec4 gScreenSize;
    vec4 gJitter;
    vec4 gDitTMAAVal;
    vec4 gmProjNonJitter[4];
    vec4 gmDiffPreMat[4];
    vec4 gLightShaft;
    vec4 gWetParam[2];
}U_Static;
layout(binding = 0, std430) buffer _U_Bone {
    uint data[];
}U_Bone;
layout(binding = 1, std430) buffer _U_OdB {
    uint data[];
}U_OdB;
layout(location = 0) in vec4 vPos;
layout(location = 1) in vec4 nWgtIdx;
layout(location = 2) in vec4 vNormal;
layout(location = 3) in vec4 vColor;
layout(location = 0) out vec4 out_attr0;
layout(location = 1) out vec4 out_attr1;
layout(location = 2) out vec4 out_attr2;
layout(location = 3) out vec4 out_attr3;
void main() {
    precise float temp_0;
    precise float temp_1;
    precise float temp_2;
    precise float temp_3;
    precise float temp_4;
    precise float temp_5;
    int temp_6;
    int temp_7;
    precise float temp_8;
    uint temp_9;
    int temp_10;
    int temp_11;
    int temp_12;
    precise float temp_13;
    precise float temp_14;
    int temp_15;
    int temp_16;
    uint temp_17;
    precise float temp_18;
    int temp_19;
    uint temp_20;
    precise float temp_21;
    int temp_22;
    uint temp_23;
    precise float temp_24;
    int temp_25;
    uint temp_26;
    precise float temp_27;
    uint temp_28;
    precise float temp_29;
    int temp_30;
    uint temp_31;
    precise float temp_32;
    int temp_33;
    uint temp_34;
    precise float temp_35;
    int temp_36;
    uint temp_37;
    precise float temp_38;
    uint temp_39;
    precise float temp_40;
    int temp_41;
    uint temp_42;
    precise float temp_43;
    int temp_44;
    uint temp_45;
    precise float temp_46;
    int temp_47;
    uint temp_48;
    precise float temp_49;
    uint temp_50;
    precise float temp_51;
    int temp_52;
    uint temp_53;
    precise float temp_54;
    int temp_55;
    uint temp_56;
    precise float temp_57;
    int temp_58;
    uint temp_59;
    precise float temp_60;
    uint temp_61;
    precise float temp_62;
    int temp_63;
    uint temp_64;
    precise float temp_65;
    int temp_66;
    uint temp_67;
    precise float temp_68;
    int temp_69;
    uint temp_70;
    precise float temp_71;
    uint temp_72;
    precise float temp_73;
    int temp_74;
    uint temp_75;
    precise float temp_76;
    int temp_77;
    uint temp_78;
    precise float temp_79;
    int temp_80;
    uint temp_81;
    precise float temp_82;
    precise float temp_83;
    precise float temp_84;
    precise float temp_85;
    precise float temp_86;
    precise float temp_87;
    precise float temp_88;
    precise float temp_89;
    precise float temp_90;
    precise float temp_91;
    precise float temp_92;
    precise float temp_93;
    precise float temp_94;
    precise float temp_95;
    precise float temp_96;
    precise float temp_97;
    precise float temp_98;
    precise float temp_99;
    precise float temp_100;
    precise float temp_101;
    precise float temp_102;
    precise float temp_103;
    precise float temp_104;
    precise float temp_105;
    precise float temp_106;
    precise float temp_107;
    precise float temp_108;
    precise float temp_109;
    precise float temp_110;
    precise float temp_111;
    precise float temp_112;
    precise float temp_113;
    precise float temp_114;
    precise float temp_115;
    precise float temp_116;
    precise float temp_117;
    precise float temp_118;
    precise float temp_119;
    precise float temp_120;
    precise float temp_121;
    precise float temp_122;
    precise float temp_123;
    precise float temp_124;
    precise float temp_125;
    precise float temp_126;
    precise float temp_127;
    precise float temp_128;
    precise float temp_129;
    precise float temp_130;
    precise float temp_131;
    precise float temp_132;
    precise float temp_133;
    precise float temp_134;
    precise float temp_135;
    precise float temp_136;
    precise float temp_137;
    precise float temp_138;
    precise float temp_139;
    precise float temp_140;
    precise float temp_141;
    precise float temp_142;
    precise float temp_143;
    precise float temp_144;
    precise float temp_145;
    precise float temp_146;
    precise float temp_147;
    precise float temp_148;
    precise float temp_149;
    precise float temp_150;
    precise float temp_151;
    precise float temp_152;
    precise float temp_153;
    precise float temp_154;
    precise float temp_155;
    precise float temp_156;
    precise float temp_157;
    precise float temp_158;
    precise float temp_159;
    precise float temp_160;
    precise float temp_161;
    precise float temp_162;
    precise float temp_163;
    precise float temp_164;
    precise float temp_165;
    precise float temp_166;
    precise float temp_167;
    precise float temp_168;
    precise float temp_169;
    precise float temp_170;
    precise float temp_171;
    precise float temp_172;
    precise float temp_173;
    precise float temp_174;
    gl_PointSize = 1.;
    gl_Position.x = 0.;
    gl_Position.y = 0.;
    gl_Position.z = 0.;
    gl_Position.w = 1.;
    temp_0 = nWgtIdx.x;
    temp_1 = vNormal.x;
    temp_2 = vPos.x;
    temp_3 = vNormal.y;
    temp_4 = vPos.y;
    temp_5 = vNormal.z;
    temp_6 = floatBitsToInt(temp_0) & 65535;
    temp_7 = temp_6 * 48;
    temp_8 = vPos.z;
    temp_9 = floatBitsToUint(temp_0) >> 16;
    temp_10 = int(temp_9) * 48;
    temp_11 = temp_10 << 16;
    temp_12 = temp_11 + temp_7;
    temp_13 = vPos.w;
    temp_14 = vColor.w;
    temp_15 = temp_12 + 32;
    temp_16 = temp_12 + 16;
    temp_17 = uint(temp_12) >> 2;
    temp_18 = uintBitsToFloat(U_Bone.data[int(temp_17)]);
    temp_19 = temp_12 + 4;
    temp_20 = uint(temp_19) >> 2;
    temp_21 = uintBitsToFloat(U_Bone.data[int(temp_20)]);
    temp_22 = temp_12 + 8;
    temp_23 = uint(temp_22) >> 2;
    temp_24 = uintBitsToFloat(U_Bone.data[int(temp_23)]);
    temp_25 = temp_12 + 12;
    temp_26 = uint(temp_25) >> 2;
    temp_27 = uintBitsToFloat(U_Bone.data[int(temp_26)]);
    temp_28 = uint(temp_15) >> 2;
    temp_29 = uintBitsToFloat(U_Bone.data[int(temp_28)]);
    temp_30 = temp_15 + 4;
    temp_31 = uint(temp_30) >> 2;
    temp_32 = uintBitsToFloat(U_Bone.data[int(temp_31)]);
    temp_33 = temp_15 + 8;
    temp_34 = uint(temp_33) >> 2;
    temp_35 = uintBitsToFloat(U_Bone.data[int(temp_34)]);
    temp_36 = temp_15 + 12;
    temp_37 = uint(temp_36) >> 2;
    temp_38 = uintBitsToFloat(U_Bone.data[int(temp_37)]);
    temp_39 = uint(temp_16) >> 2;
    temp_40 = uintBitsToFloat(U_Bone.data[int(temp_39)]);
    temp_41 = temp_16 + 4;
    temp_42 = uint(temp_41) >> 2;
    temp_43 = uintBitsToFloat(U_Bone.data[int(temp_42)]);
    temp_44 = temp_16 + 8;
    temp_45 = uint(temp_44) >> 2;
    temp_46 = uintBitsToFloat(U_Bone.data[int(temp_45)]);
    temp_47 = temp_16 + 12;
    temp_48 = uint(temp_47) >> 2;
    temp_49 = uintBitsToFloat(U_Bone.data[int(temp_48)]);
    temp_50 = uint(temp_12) >> 2;
    temp_51 = uintBitsToFloat(U_OdB.data[int(temp_50)]);
    temp_52 = temp_12 + 4;
    temp_53 = uint(temp_52) >> 2;
    temp_54 = uintBitsToFloat(U_OdB.data[int(temp_53)]);
    temp_55 = temp_12 + 8;
    temp_56 = uint(temp_55) >> 2;
    temp_57 = uintBitsToFloat(U_OdB.data[int(temp_56)]);
    temp_58 = temp_12 + 12;
    temp_59 = uint(temp_58) >> 2;
    temp_60 = uintBitsToFloat(U_OdB.data[int(temp_59)]);
    temp_61 = uint(temp_16) >> 2;
    temp_62 = uintBitsToFloat(U_OdB.data[int(temp_61)]);
    temp_63 = temp_16 + 4;
    temp_64 = uint(temp_63) >> 2;
    temp_65 = uintBitsToFloat(U_OdB.data[int(temp_64)]);
    temp_66 = temp_16 + 8;
    temp_67 = uint(temp_66) >> 2;
    temp_68 = uintBitsToFloat(U_OdB.data[int(temp_67)]);
    temp_69 = temp_16 + 12;
    temp_70 = uint(temp_69) >> 2;
    temp_71 = uintBitsToFloat(U_OdB.data[int(temp_70)]);
    temp_72 = uint(temp_15) >> 2;
    temp_73 = uintBitsToFloat(U_OdB.data[int(temp_72)]);
    temp_74 = temp_15 + 4;
    temp_75 = uint(temp_74) >> 2;
    temp_76 = uintBitsToFloat(U_OdB.data[int(temp_75)]);
    temp_77 = temp_15 + 8;
    temp_78 = uint(temp_77) >> 2;
    temp_79 = uintBitsToFloat(U_OdB.data[int(temp_78)]);
    temp_80 = temp_15 + 12;
    temp_81 = uint(temp_80) >> 2;
    temp_82 = uintBitsToFloat(U_OdB.data[int(temp_81)]);
    temp_83 = temp_14 * U_Mate.gWrkFl4[0].z;
    temp_84 = temp_18 * temp_1;
    temp_85 = fma(temp_21, temp_3, temp_84);
    temp_86 = temp_29 * temp_2;
    temp_87 = temp_29 * temp_1;
    temp_88 = temp_40 * temp_1;
    temp_89 = fma(temp_32, temp_4, temp_86);
    temp_90 = fma(temp_24, temp_5, temp_85);
    temp_91 = fma(temp_43, temp_3, temp_88);
    temp_92 = fma(temp_32, temp_3, temp_87);
    temp_93 = temp_90 * U_Toon2.gToonParam[2].x;
    temp_94 = fma(temp_46, temp_5, temp_91);
    temp_95 = fma(temp_35, temp_8, temp_89);
    temp_96 = fma(temp_35, temp_5, temp_92);
    temp_97 = fma(temp_94, U_Toon2.gToonParam[2].y, temp_93);
    temp_98 = 1. / U_Static.gmProj[1].y;
    temp_99 = temp_18 * temp_2;
    temp_100 = vColor.x;
    temp_101 = fma(temp_38, temp_13, temp_95);
    temp_102 = fma(temp_96, U_Toon2.gToonParam[2].z, temp_97);
    temp_103 = fma(temp_21, temp_4, temp_99);
    temp_104 = vColor.y;
    temp_105 = 0. - temp_83;
    temp_106 = temp_101 * temp_105;
    temp_107 = fma(temp_102, 0.5, 0.5);
    temp_108 = clamp(temp_107, 0., 1.);
    temp_109 = fma(temp_24, temp_8, temp_103);
    temp_110 = temp_40 * temp_2;
    temp_111 = vColor.z;
    temp_112 = temp_106 * temp_98;
    out_attr0.x = temp_90;
    temp_113 = 0. - U_Toon2.gToonParam[3].z;
    temp_114 = temp_108 * temp_113;
    out_attr0.y = temp_94;
    temp_115 = fma(temp_27, temp_13, temp_109);
    out_attr1.x = temp_100;
    temp_116 = fma(temp_43, temp_4, temp_110);
    out_attr1.y = temp_104;
    temp_117 = fma(temp_112, temp_114, temp_112);
    out_attr1.z = temp_111;
    temp_118 = temp_51 * temp_2;
    out_attr1.w = temp_117;
    temp_119 = fma(temp_46, temp_8, temp_116);
    temp_120 = temp_62 * temp_2;
    temp_121 = fma(temp_54, temp_4, temp_118);
    temp_122 = temp_73 * temp_2;
    temp_123 = fma(temp_65, temp_4, temp_120);
    temp_124 = fma(temp_57, temp_8, temp_121);
    temp_125 = fma(temp_76, temp_4, temp_122);
    temp_126 = fma(temp_68, temp_8, temp_123);
    temp_127 = fma(temp_60, temp_13, temp_124);
    temp_128 = fma(temp_79, temp_8, temp_125);
    temp_129 = fma(temp_71, temp_13, temp_126);
    temp_130 = fma(temp_90, temp_117, temp_115);
    temp_131 = fma(temp_49, temp_13, temp_119);
    temp_132 = fma(temp_90, temp_117, temp_127);
    temp_133 = fma(temp_82, temp_13, temp_128);
    temp_134 = fma(temp_94, temp_117, temp_129);
    temp_135 = temp_130 * U_Static.gmProj[3].x;
    temp_136 = temp_130 * U_Static.gmProj[0].x;
    temp_137 = temp_130 * U_Static.gmProj[1].x;
    temp_138 = temp_132 * U_Static.gmProj[2].x;
    temp_139 = fma(temp_94, temp_117, temp_131);
    temp_140 = temp_130 * U_Static.gmProj[2].x;
    temp_141 = fma(temp_96, temp_117, temp_101);
    temp_142 = fma(temp_96, temp_117, temp_133);
    temp_143 = fma(temp_134, U_Static.gmProj[2].y, temp_138);
    temp_144 = temp_132 * U_Static.gmProj[1].x;
    temp_145 = fma(temp_139, U_Static.gmProj[0].y, temp_136);
    temp_146 = fma(temp_139, U_Static.gmProj[2].y, temp_140);
    temp_147 = fma(temp_139, U_Static.gmProj[3].y, temp_135);
    temp_148 = fma(temp_139, U_Static.gmProj[1].y, temp_137);
    temp_149 = temp_132 * U_Static.gmProj[3].x;
    temp_150 = fma(temp_134, U_Static.gmProj[1].y, temp_144);
    temp_151 = fma(temp_141, U_Static.gmProj[0].z, temp_145);
    temp_152 = temp_132 * U_Static.gmProj[0].x;
    temp_153 = fma(temp_141, U_Static.gmProj[2].z, temp_146);
    temp_154 = fma(temp_134, U_Static.gmProj[3].y, temp_149);
    temp_155 = fma(temp_142, U_Static.gmProj[1].z, temp_150);
    temp_156 = temp_151 + U_Static.gmProj[0].w;
    temp_157 = fma(temp_134, U_Static.gmProj[0].y, temp_152);
    gl_Position.x = temp_156;
    temp_158 = temp_153 + U_Static.gmProj[2].w;
    out_attr2.x = temp_156;
    temp_159 = fma(temp_141, U_Static.gmProj[3].z, temp_147);
    gl_Position.z = temp_158;
    temp_160 = fma(temp_141, U_Static.gmProj[1].z, temp_148);
    temp_161 = fma(temp_142, U_Static.gmProj[3].z, temp_154);
    temp_162 = fma(temp_142, U_Static.gmProj[2].z, temp_143);
    temp_163 = temp_155 + U_Static.gmProj[1].w;
    temp_164 = fma(temp_142, U_Static.gmProj[0].z, temp_157);
    out_attr3.y = temp_163;
    temp_165 = 0. - U_Static.gCDep.y;
    temp_166 = temp_158 + temp_165;
    temp_167 = abs(temp_96);
    temp_168 = temp_167 + -0.;
    temp_169 = temp_159 + U_Static.gmProj[3].w;
    out_attr0.z = temp_168;
    temp_170 = temp_160 + U_Static.gmProj[1].w;
    gl_Position.w = temp_169;
    temp_171 = temp_161 + U_Static.gmProj[3].w;
    out_attr2.w = temp_169;
    temp_172 = temp_162 + U_Static.gmProj[2].w;
    gl_Position.y = temp_170;
    temp_173 = temp_164 + U_Static.gmProj[0].w;
    out_attr2.y = temp_170;
    temp_174 = temp_166 * U_Static.gCDep.z;
    out_attr3.w = temp_171;
    out_attr3.z = temp_172;
    out_attr3.x = temp_173;
    out_attr2.z = temp_174;
    return;
}
