//
//  HomePageNotchView.swift
//  DynamicNotch
//
//  Created by Евгений Петрукович on 5/18/26.
//

import SwiftUI

struct HomePageNotchView: View {
    @Environment(\.isNotchlessScreen) var isNotchlessScreen
    
    let notchViewModel: NotchViewModel
    let settings: HomePageSettingsStore
    let localTimerViewModel: LocalTimerViewModel
    let nowPlayingViewModel: NowPlayingViewModel
    let mediaAndFilesSettings: MediaAndFilesSettingsStore
    let applicationSettings: ApplicationSettingsStore
    let initialPage: HomePages
    
    @State private var currentPage: HomePages?
    @State private var updateTask: Task<Void, Never>? = nil
    @State private var isWaitingForSizeUpdate = false
    @State private var isPageSettled = true
    @State private var settleTask: Task<Void, Never>? = nil
    
    init(notchViewModel: NotchViewModel, settings: HomePageSettingsStore, localTimerViewModel: LocalTimerViewModel, nowPlayingViewModel: NowPlayingViewModel, mediaAndFilesSettings: MediaAndFilesSettingsStore, applicationSettings: ApplicationSettingsStore, initialPage: HomePages) {
        self.notchViewModel = notchViewModel
        self.settings = settings
        self.localTimerViewModel = localTimerViewModel
        self.nowPlayingViewModel = nowPlayingViewModel
        self.mediaAndFilesSettings = mediaAndFilesSettings
        self.applicationSettings = applicationSettings
        self.initialPage = initialPage
        
        let activePages = settings.homePageOrder.filter { !settings.homePageDisabled.contains($0) }
        let pageToSelect = activePages.contains(initialPage) ? initialPage : (activePages.first ?? .camera)
        self._currentPage = State(initialValue: pageToSelect)
    }
    
    var body: some View {
        let activePages = settings.homePageOrder.filter { !settings.homePageDisabled.contains($0) }
        let settled = isPageSettled
        
        VStack {
            ScrollViewReader { proxy in
                ScrollView(.vertical, showsIndicators: false) {
                    VStack(spacing: 20) {
                        ForEach(activePages) { page in
                            pageView(for: page)
                                .containerRelativeFrame(.vertical)
                                .scrollTransition(.interactive) { content, phase in
                                    content
                                        .blur(radius: settled ? min(20, CGFloat(abs(phase.value)) * 150) : 20)
                                        .opacity(settled ? max(0.7, 1.0 - (abs(phase.value) * 2.0)) : 0.7)
                                }
                                .id(page)
                        }
                    }
                    .scrollTargetLayout()
                }
                .scrollTargetBehavior(.viewAligned)
                .scrollPosition(id: $currentPage, anchor: .top)
                .onAppear {
                    let targetPage = activePages.contains(initialPage) ? initialPage : (activePages.first ?? .camera)
                    currentPage = targetPage
                    updateLastPageStatus(page: targetPage, activePages: activePages)
                    proxy.scrollTo(targetPage, anchor: .top)

                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
                        proxy.scrollTo(targetPage, anchor: .top)
                    }
                }
                .onChange(of: notchViewModel.presentedNotchSize.height) { _, _ in
                    if let current = currentPage {
                        proxy.scrollTo(current, anchor: .top)
                    }
                }
                .onChange(of: initialPage) { _, newPage in
                    if newPage != currentPage && activePages.contains(newPage) {
                        currentPage = newPage
                        withAnimation(.spring(response: 0.4, dampingFraction: 0.85)) {
                            proxy.scrollTo(newPage, anchor: .top)
                        }
                    }
                }
            }
            .mask {
                if !isNotchlessScreen && !notchViewModel.isDynamicIsland {
                    let baseHeight = notchViewModel.notchModel.baseHeight
                    let cornerRadius: CGFloat = 30
                    
                    RoundedRectangle(cornerRadius: cornerRadius)
                        .mask(
                            VStack(spacing: 0) {
                                Color.clear
                                    .frame(height: baseHeight)
                                LinearGradient(
                                    stops: [
                                        .init(color: .clear, location: 0),
                                        .init(color: .black, location: 1)
                                    ],
                                    startPoint: .top,
                                    endPoint: .bottom
                                )
                                .frame(height: 4)
                                Color.black
                            }
                        )
                } else {
                    Color.black
                }
            }
        }
        .onChange(of: activePages) { _, newActivePages in
            let updatedPage: HomePages?
            if let current = currentPage, !newActivePages.contains(current) {
                let first = newActivePages.first
                currentPage = first
                updatedPage = first
            } else {
                updatedPage = currentPage
            }
            updateLastPageStatus(page: updatedPage, activePages: newActivePages)
        }
        .onChange(of: currentPage) { oldPage, newPage in
            updateLastPageStatus(page: newPage, activePages: activePages)
            guard let oldPage = oldPage, let newPage = newPage, newPage != oldPage else { return }
            
            withAnimation(.easeInOut(duration: 0.15)) {
                isWaitingForSizeUpdate = true
            }
            
            isPageSettled = false
            settleTask?.cancel()
            settleTask = Task {
                try? await Task.sleep(nanoseconds: 500_000_000)
                guard !Task.isCancelled else { return }
                withAnimation(.spring(response: 0.5)) {
                    isPageSettled = true
                }
            }
            
            updateTask?.cancel()
            updateTask = Task {
                try? await Task.sleep(nanoseconds: 300_000_000)
                guard !Task.isCancelled else { return }
                
                notchViewModel.send(
                    .showLiveActivity(
                        HomePageNotchContent(
                            notchViewModel: notchViewModel,
                            settings: settings,
                            homePages: newPage,
                            localTimerViewModel: localTimerViewModel,
                            nowPlayingViewModel: nowPlayingViewModel,
                            mediaAndFilesSettings: mediaAndFilesSettings,
                            applicationSettings: applicationSettings
                        )
                    )
                )
                
                withAnimation(.easeInOut(duration: 0.35)) {
                    isWaitingForSizeUpdate = false
                }
            }
        }
        .onDisappear {
            notchViewModel.isHomePageOnLastPage = false
            let activePages = settings.homePageOrder.filter { !settings.homePageDisabled.contains($0) }
            notchViewModel.send(
                .showLiveActivity(
                    HomePageNotchContent(
                        notchViewModel: notchViewModel,
                        settings: settings,
                        homePages: activePages.first ?? .camera,
                        localTimerViewModel: localTimerViewModel,
                        nowPlayingViewModel: nowPlayingViewModel,
                        mediaAndFilesSettings: mediaAndFilesSettings,
                        applicationSettings: applicationSettings
                    )
                )
            )
            settleTask?.cancel()
            updateTask?.cancel()
        }
    }
    
    private func updateLastPageStatus(page: HomePages?, activePages: [HomePages]) {
        let isLast = page != nil && page == activePages.last
        if notchViewModel.isHomePageOnLastPage != isLast {
            notchViewModel.isHomePageOnLastPage = isLast
        }
    }
    
    @ViewBuilder
    private func pageView(for page: HomePages) -> some View {
        switch page {
        case .camera:
            CameraNotchView(notchViewModel: notchViewModel, settings: settings, localTimerViewModel: localTimerViewModel, nowPlayingViewModel: nowPlayingViewModel, mediaAndFilesSettings: mediaAndFilesSettings, applicationSettings: applicationSettings)
        case .localTimer:
            LocalTimerSetupNotchView(localTimerViewModel: localTimerViewModel)
        case .vpn:
            VpnPageNotchView(notchViewModel: notchViewModel)
        }
    }
}
