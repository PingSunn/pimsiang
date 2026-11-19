# PimSiang (พิมพ์เสียง)

<a href="https://openthai.ai/openthai2p0-asr-streaming?ref=powered-by" target="_blank" rel="noopener"><img src="https://openthai.ai/img/badges/powered-by-openthai-asr.svg?v=2" alt="Powered by OpenThai ASR" height="32"></a>

โปรแกรมพิมพ์ด้วยเสียงภาษาไทยสำหรับ macOS กดปุ่มค้างไว้ พูดภาษาไทย แล้วปล่อย ข้อความจะถูกพิมพ์ลงในแอปที่คุณกำลังใช้อยู่ การรู้จำเสียงทำงานบนเครื่อง Mac ของคุณเอง

รองรับ Apple Silicon และ macOS 14 (Sonoma) ขึ้นไป [English](README.md)

## ติดตั้ง

```sh
brew install --cask PingSunn/tap/pimsiang
```

อัปเกรดภายหลังด้วย `brew upgrade --cask PingSunn/tap/pimsiang` แอปจะแจ้งที่เมนูบาร์เมื่อมีเวอร์ชันใหม่ แต่จะไม่อัปเดตตัวเอง

### Open Anyway (ทำครั้งเดียว)

PimSiang เซ็นด้วยใบรับรองที่สร้างเอง (self-signed) และไม่ได้ผ่าน notarization ของ Apple ดังนั้น macOS จะบล็อกการเปิดครั้งแรก

1. เปิด PimSiang จาก Applications macOS จะแจ้งว่าตรวจสอบแอปไม่ได้ ให้กด Done
2. เปิด System Settings แล้วไปที่ Privacy & Security เลื่อนลงไปที่ Security แล้วกด **Open Anyway** ข้าง PimSiang
3. ยืนยันด้วยรหัสผ่านหรือ Touch ID แล้วเปิดแอป

ทำครั้งเดียวพอ เมื่ออัปเกรดแล้วสิทธิ์ที่เคยอนุญาตไว้ยังใช้ได้

## สิทธิ์ที่ต้องอนุญาต

PimSiang ขอสิทธิ์สามอย่าง และจะบอกจุดที่ต้องไปอนุญาตตอนเปิดครั้งแรก

- **Microphone**: เพื่อฟังเสียงขณะกดปุ่ม Trigger ค้างไว้
- **Accessibility**: เพื่อหาช่องข้อความที่กำลังใช้งานและวางข้อความ
- **Input Monitoring**: เพื่อรับรู้ปุ่ม Trigger (ฟังเฉยๆ ไม่บันทึกปุ่มอื่น)

## เปิดครั้งแรก: ดาวน์โหลด Model

Model รู้จำเสียงมีขนาดประมาณ 1 GB ดาวน์โหลดตอนเปิดครั้งแรกจาก Hugging Face (ไฟล์ตรวจจับเสียงพูด Silero จาก GitHub) ตรวจด้วย checksum และเก็บไว้ใน `~/Library/Application Support/PimSiang` เริ่มพิมพ์ด้วยเสียงได้เมื่อดาวน์โหลดเสร็จ ถ้าเลือก "แม่นยำ" ใน Settings › ทั่วไป แอปจะดาวน์โหลด Model ขนาดเต็ม (ประมาณ 2.6 GB และใช้หน่วยความจำมากกว่า) ไว้เบื้องหลัง แล้วสลับไปใช้เมื่อพร้อม

## วิธีใช้

- **กด Option ขวาค้างไว้** พูด แล้วปล่อย: ข้อความจะถูกพิมพ์ตรงตำแหน่งเคอร์เซอร์
- **แตะ Option ขวาสองครั้ง** เพื่อเริ่มโหมด Hands-free แล้วแตะอีกครั้งเพื่อจบ
- **กด Shift ร่วมกับ Option ขวา** สำหรับ Quick note: ข้อความจะถูกเก็บไว้ใน History แทนการพิมพ์
- **Esc** ยกเลิก Utterance ปัจจุบัน

ถ้าไม่มีช่องข้อความที่กำลังโฟกัสอยู่ ข้อความจะถูกเก็บเป็น Note ใน History เปลี่ยนปุ่ม Trigger ได้ในหน้าตั้งค่า

## ความเป็นส่วนตัว

- ไม่เก็บเสียง และเสียงไม่ออกจากเครื่อง Mac ของคุณ
- History เก็บเฉพาะข้อความ อยู่บนเครื่องของคุณ
- ไม่มี telemetry และไม่ส่งรายงานข้อผิดพลาด
- การเชื่อมต่ออินเทอร์เน็ตมีเพียงการดาวน์โหลด Model (Hugging Face และ GitHub) และการเช็ก GitHub Releases วันละครั้งเพื่อดูว่ามีเวอร์ชันใหม่หรือไม่

## ถอนการติดตั้ง

```sh
brew uninstall --cask PingSunn/tap/pimsiang
```

ถ้าต้องการลบ Model, History, การตั้งค่า และ log ด้วย:

```sh
brew uninstall --cask --zap PingSunn/tap/pimsiang
```

## สัญญาอนุญาต

PimSiang ใช้สัญญาอนุญาต MIT (ดูไฟล์ `LICENSE`) ส่วนประกอบที่รวมมาหรือดาวน์โหลดมาใช้สัญญาอนุญาตของตัวเอง: Model OpenThai 2.0 ASR (iApp) ภายใต้ OpenMDW-1.1, Silero VAD ภายใต้ MIT และฟอนต์ Anuphan ภายใต้ SIL Open Font License 1.1

## การพัฒนา

`scripts/check.sh` รัน format, build, clippy และ test ส่วน `scripts/make-signing-cert.sh` สร้าง certificate สำหรับ sign บนเครื่องครั้งเดียว และ `scripts/release.sh X.Y.Z --dry-run` จะ build, sign และ zip release ไว้ใน `target/release-dist/` โดยไม่เผยแพร่
