//! A compositor clip cuts both chart strokes and fill at the actual tile contour.
use super::*;
use crate::presentation::history::ChartPoint;
use std::ptr;

pub(super) struct RoundedClip;
impl RoundedClip {
    pub(super) fn apply(element: &Com, width: f32, height: f32, radius: f32) -> Result<()> {
        let preview = factory(
            "Windows.UI.Xaml.Hosting.ElementCompositionPreview",
            &Guid::from_u128(0x08c92b38_ec99_4c55_bc85_a1c180b27646),
        )?;
        let element = element.query(&UI_ELEMENT)?;
        unsafe {
            let get: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = preview.slot(6);
            let mut raw = ptr::null_mut();
            check(get(preview.raw(), element.raw(), &mut raw))?;
            let visual = Com::owned(raw)?;
            let compositor = visual
                .query(&Guid::from_u128(0xbcb4ad45_7609_4550_934f_16002a68fded))?
                .object(6)?;
            let geometry = compositor
                .query(&Guid::from_u128(0x48ea31ad_7fcd_4076_a79c_90cc4b852c9b))?
                .object(20)?;
            // Windows.Foundation.Numerics.Vector2 has the same two-f32 ABI as Point.
            let size: unsafe extern "system" fn(Raw, ChartPoint) -> Hr = geometry.slot(11);
            let corners: unsafe extern "system" fn(Raw, ChartPoint) -> Hr = geometry.slot(7);
            check(size(
                geometry.raw(),
                ChartPoint {
                    x: width,
                    y: height,
                },
            ))?;
            check(corners(
                geometry.raw(),
                ChartPoint {
                    x: radius,
                    y: radius,
                },
            ))?;
            let geometry =
                geometry.query(&Guid::from_u128(0xe985217c_6a17_4207_abd8_5fd3dd612a9d))?;
            let compositor =
                compositor.query(&Guid::from_u128(0x7a38b2bd_cec8_4eeb_830f_d8d07aedebc3))?;
            let create: unsafe extern "system" fn(Raw, Raw, *mut Raw) -> Hr = compositor.slot(7);
            let mut raw = ptr::null_mut();
            check(create(compositor.raw(), geometry.raw(), &mut raw))?;
            let clip =
                Com::owned(raw)?.query(&Guid::from_u128(0x1ccd2a52_cfc7_4ace_9983_146bb8eb6a3c))?;
            let set: unsafe extern "system" fn(Raw, Raw) -> Hr = visual.slot(15);
            // The visual retains the clip, and the clip retains its geometry.
            check(set(visual.raw(), clip.raw()))
        }
    }
}
