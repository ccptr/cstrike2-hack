#[cfg(feature = "directx11")]
pub mod dx11 {
    use crate::{
        get_original_fn,
        utils::render,
    };

    use windows::{
        core::HRESULT,
        Win32::Graphics::Dxgi::{Common::DXGI_FORMAT, IDXGISwapChain},
    };

    extern "system" fn hk_present(
        swapchain: IDXGISwapChain,
        sync_interval: u32,
        flags: u32,
    ) -> HRESULT {
        get_original_fn!(hk_present, original_fn, (IDXGISwapChain, u32, u32), HRESULT);

        render::dx11::init_from_swapchain(&swapchain);

        original_fn(swapchain, sync_interval, flags)
    }

    extern "system" fn hk_resize_buffers(
        swapchain: IDXGISwapChain,
        buffer_count: u32,
        width: u32,
        height: u32,
        new_format: DXGI_FORMAT,
        swapchain_flags: u32,
    ) -> HRESULT {
        get_original_fn!(
            hk_resize_buffers,
            original_fn,
            (IDXGISwapChain, u32, u32, u32, DXGI_FORMAT, u32),
            HRESULT
        );

        let mut renderer = render::dx11::DX11
            .get()
            .expect("dx11 renderer is not initialized while resizing buffers")
            .lock();

        renderer
            .resize_buffers(&swapchain, || {
                original_fn(swapchain.clone(), buffer_count, width, height, new_format, swapchain_flags)
            })
        .expect("could not resize buffers")
    }
}
